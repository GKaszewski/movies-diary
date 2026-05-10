use std::sync::Arc;

use anyhow::Context;
use event_publisher::{EventPublisherConfig, NoopEventPublisher, create_event_channel};
use presentation::event_handlers::PosterSyncHandler;
use std::str::FromStr;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "sqlite")]
use sqlite::{SqliteMovieRepository, SqliteUserRepository};
#[cfg(feature = "sqlite-federation")]
use sqlite_federation::SqliteFederationRepository;

#[cfg(feature = "postgres")]
use postgres::{PostgresRepository, PostgresUserRepository};
#[cfg(feature = "postgres-federation")]
use postgres_federation::PostgresFederationRepository;

#[cfg(feature = "federation")]
use activitypub::{
    ActivityPubEventHandler, ActivityPubPort, ActivityPubService, DomainUserRepoAdapter,
    ReviewObjectHandler,
};

use application::{config::AppConfig, context::AppContext};
use auth::{Argon2PasswordHasher, AuthConfig, JwtAuthService};
use export::ExportAdapter;
use metadata::MetadataClientImpl;
use poster_fetcher::{PosterFetcherConfig, ReqwestPosterFetcher};
use poster_storage::{PosterStorageAdapter, StorageConfig};
use rss::RssAdapter;
use template_askama::AskamaHtmlRenderer;

use doc::ApiDocExt;
use presentation::{openapi::ApiDoc, routes, state::AppState};
use utoipa::OpenApi as _;

use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, MetadataClient, MovieRepository,
    PasswordHasher, PosterFetcherClient, PosterStorage, ReviewRepository, StatsRepository,
    UserRepository,
};

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("At least one database backend must be enabled. Use --features sqlite or --features postgres");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let (state, ap_router) = wire_dependencies()
        .await
        .context("Failed to wire dependencies")?;

    let app = routes::build_router(state, ap_router).with_api_doc(ApiDoc::openapi());

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<(AppState, axum::Router)> {
    let auth_config = AuthConfig::from_env()?;
    let storage_config = StorageConfig::from_env()?;
    let app_config = AppConfig::from_env();
    let omdb_api_key = std::env::var("OMDB_API_KEY").context("OMDB_API_KEY must be set")?;

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());

    let metadata_client: Arc<dyn MetadataClient> =
        Arc::new(MetadataClientImpl::new_omdb(omdb_api_key));
    let poster_fetcher: Arc<dyn PosterFetcherClient> =
        Arc::new(ReqwestPosterFetcher::new(PosterFetcherConfig::from_env())?);
    let poster_storage: Arc<dyn PosterStorage> =
        Arc::new(PosterStorageAdapter::from_config(storage_config));
    let auth_service: Arc<dyn AuthService> = Arc::new(JwtAuthService::new(auth_config));
    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);

    // Only track pools when the federation feature for that backend needs them
    #[cfg(feature = "sqlite-federation")]
    let mut sqlite_pool: Option<sqlx::SqlitePool> = None;
    #[cfg(feature = "postgres-federation")]
    let mut pg_pool: Option<sqlx::PgPool> = None;

    let (movie_repository, review_repository, diary_repository, stats_repository, user_repository):
        (Arc<dyn MovieRepository>, Arc<dyn ReviewRepository>, Arc<dyn DiaryRepository>,
         Arc<dyn StatsRepository>, Arc<dyn UserRepository>) =
        match backend.as_str() {
            #[cfg(feature = "postgres")]
            "postgres" => {
                let (_pool, m, r, d, s, u) = wire_postgres(&database_url).await?;
                #[cfg(feature = "postgres-federation")]
                { pg_pool = Some(_pool); }
                (m, r, d, s, u)
            }
            #[cfg(feature = "sqlite")]
            _ => {
                let (_pool, m, r, d, s, u) = wire_sqlite(&database_url).await?;
                #[cfg(feature = "sqlite-federation")]
                { sqlite_pool = Some(_pool); }
                (m, r, d, s, u)
            }
            #[cfg(not(feature = "sqlite"))]
            _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build (sqlite feature is not enabled)"),
        };

    // Build handler context (used for poster sync handler)
    let handler_ctx = AppContext {
        movie_repository: Arc::clone(&movie_repository),
        review_repository: Arc::clone(&review_repository),
        diary_repository: Arc::clone(&diary_repository),
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        stats_repository: Arc::clone(&stats_repository),
        metadata_client: Arc::clone(&metadata_client),
        poster_fetcher: Arc::clone(&poster_fetcher),
        poster_storage: Arc::clone(&poster_storage),
        event_publisher: Arc::new(NoopEventPublisher),
        auth_service: Arc::clone(&auth_service),
        password_hasher: Arc::clone(&password_hasher),
        user_repository: Arc::clone(&user_repository),
        config: app_config.clone(),
    };

    // Wire up event channel, federation service, and ap_router
    #[cfg(feature = "federation")]
    let (event_publisher_arc, ap_router, ap_service, social_query) = {
        let (federation_repo, social_query_arc, review_store): (
            Arc<dyn activitypub::FederationRepository>,
            Arc<dyn domain::ports::SocialQueryPort>,
            Arc<dyn activitypub::RemoteReviewRepository>,
        ) = match backend.as_str() {
            #[cfg(feature = "postgres-federation")]
            "postgres" => {
                let pool = pg_pool.as_ref().unwrap().clone();
                let fed = Arc::new(PostgresFederationRepository::new(pool));
                (Arc::clone(&fed) as _, Arc::clone(&fed) as _, fed as _)
            }
            #[cfg(feature = "sqlite-federation")]
            _ => {
                let pool = sqlite_pool.as_ref().unwrap().clone();
                let fed = Arc::new(SqliteFederationRepository::new(pool));
                (Arc::clone(&fed) as _, Arc::clone(&fed) as _, fed as _)
            }
            #[cfg(not(feature = "sqlite-federation"))]
            _ => anyhow::bail!("DATABASE_BACKEND={backend} federation is not supported by this build"),
        };

        let user_repo_adapter = Arc::new(DomainUserRepoAdapter(Arc::clone(&user_repository)));
        let review_handler = Arc::new(ReviewObjectHandler {
            movie_repository: Arc::clone(&movie_repository),
            diary_repository: Arc::clone(&diary_repository),
            review_store,
            base_url: app_config.base_url.clone(),
        });
        let concrete_ap_service = Arc::new(
            ActivityPubService::new(
                federation_repo,
                user_repo_adapter,
                review_handler,
                app_config.base_url.clone(),
                cfg!(debug_assertions),
            )
            .await?,
        );
        let ap_router = concrete_ap_service.router();
        let ap_event_handler = ActivityPubEventHandler::new(
            Arc::clone(&concrete_ap_service),
            Arc::clone(&movie_repository),
            Arc::clone(&review_repository),
            app_config.base_url.clone(),
        );
        let ap_service_arc: Arc<dyn ActivityPubPort> = concrete_ap_service;

        let poster_handler = PosterSyncHandler::new(handler_ctx, 3);
        let (event_publisher, event_worker) = create_event_channel(
            EventPublisherConfig::from_env(),
            vec![Box::new(poster_handler), Box::new(ap_event_handler)],
        );
        tokio::spawn(event_worker.run());

        let ep: Arc<dyn domain::ports::EventPublisher> = Arc::new(event_publisher);
        (ep, ap_router, ap_service_arc, social_query_arc)
    };

    #[cfg(not(feature = "federation"))]
    let (event_publisher_arc, ap_router): (Arc<dyn domain::ports::EventPublisher>, axum::Router) = {
        let poster_handler = PosterSyncHandler::new(handler_ctx, 3);
        let (event_publisher, event_worker) = create_event_channel(
            EventPublisherConfig::from_env(),
            vec![Box::new(poster_handler)],
        );
        tokio::spawn(event_worker.run());
        (Arc::new(event_publisher), axum::Router::new())
    };

    let app_ctx = AppContext {
        movie_repository,
        review_repository,
        diary_repository,
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        stats_repository,
        metadata_client,
        poster_fetcher,
        poster_storage,
        event_publisher: event_publisher_arc,
        auth_service,
        password_hasher,
        user_repository,
        config: app_config,
    };

    let state = AppState {
        app_ctx,
        html_renderer: Arc::new(AskamaHtmlRenderer::new()),
        rss_renderer: Arc::new(RssAdapter::new(
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
        )),
        #[cfg(feature = "federation")]
        ap_service,
        #[cfg(feature = "federation")]
        social_query,
    };
    Ok((state, ap_router))
}

#[cfg(feature = "sqlite")]
async fn wire_sqlite(database_url: &str) -> anyhow::Result<(
    sqlx::SqlitePool,
    Arc<dyn MovieRepository>,
    Arc<dyn ReviewRepository>,
    Arc<dyn DiaryRepository>,
    Arc<dyn StatsRepository>,
    Arc<dyn UserRepository>,
)> {
    use sqlx::sqlite::SqliteConnectOptions;

    let opts = SqliteConnectOptions::from_str(database_url)
        .context("Invalid DATABASE_URL")?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::SqlitePool::connect_with(opts)
        .await
        .context("Failed to connect to SQLite database")?;

    let sqlite_repo = Arc::new(SqliteMovieRepository::new(pool.clone()));
    sqlite_repo
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Database migration failed")?;

    let movie_repository: Arc<dyn MovieRepository> = Arc::clone(&sqlite_repo) as _;
    let review_repository: Arc<dyn ReviewRepository> = Arc::clone(&sqlite_repo) as _;
    let diary_repository: Arc<dyn DiaryRepository> = Arc::clone(&sqlite_repo) as _;
    let stats_repository: Arc<dyn StatsRepository> = Arc::clone(&sqlite_repo) as _;
    let user_repository: Arc<dyn UserRepository> =
        Arc::new(SqliteUserRepository::new(pool.clone()));

    Ok((pool, movie_repository, review_repository, diary_repository, stats_repository, user_repository))
}

#[cfg(feature = "postgres")]
async fn wire_postgres(database_url: &str) -> anyhow::Result<(
    sqlx::PgPool,
    Arc<dyn MovieRepository>,
    Arc<dyn ReviewRepository>,
    Arc<dyn DiaryRepository>,
    Arc<dyn StatsRepository>,
    Arc<dyn UserRepository>,
)> {
    let pool = sqlx::PgPool::connect(database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    let pg_repo = Arc::new(PostgresRepository::new(pool.clone()));
    pg_repo
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Database migration failed")?;

    let movie_repository: Arc<dyn MovieRepository> = Arc::clone(&pg_repo) as _;
    let review_repository: Arc<dyn ReviewRepository> = Arc::clone(&pg_repo) as _;
    let diary_repository: Arc<dyn DiaryRepository> = Arc::clone(&pg_repo) as _;
    let stats_repository: Arc<dyn StatsRepository> = Arc::clone(&pg_repo) as _;
    let user_repository: Arc<dyn UserRepository> =
        Arc::new(PostgresUserRepository::new(pool.clone()));

    Ok((pool, movie_repository, review_repository, diary_repository, stats_repository, user_repository))
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "presentation=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
