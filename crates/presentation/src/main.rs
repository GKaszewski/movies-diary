use std::sync::Arc;

use anyhow::Context;
use event_publisher::{EventPublisherConfig, NoopEventPublisher, create_event_channel};
use presentation::event_handlers::PosterSyncHandler;
use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
use sqlite::{SqliteMovieRepository, SqliteUserRepository};
use sqlite_federation::SqliteFederationRepository;
use template_askama::AskamaHtmlRenderer;

use doc::ApiDocExt;
use presentation::{openapi::ApiDoc, routes, state::AppState};
use utoipa::OpenApi as _;

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
    axum::serve(listener, app).await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<(AppState, axum::Router)> {
    let auth_config = AuthConfig::from_env()?;
    let storage_config = StorageConfig::from_env()?;
    let app_config = AppConfig::from_env();
    let omdb_api_key = std::env::var("OMDB_API_KEY").context("OMDB_API_KEY must be set")?;

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let opts = SqliteConnectOptions::from_str(&database_url)
        .context("Invalid DATABASE_URL")?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = SqlitePool::connect_with(opts)
        .await
        .context("Failed to connect to SQLite database")?;

    let sqlite_repo = Arc::new(SqliteMovieRepository::new(pool.clone()));
    sqlite_repo
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Database migration failed")?;

    use domain::ports::{
        AuthService, DiaryExporter, DiaryRepository, MetadataClient, MovieRepository,
        PasswordHasher, PosterFetcherClient, PosterStorage, ReviewRepository, StatsRepository,
        UserRepository,
    };
    let movie_repository: Arc<dyn MovieRepository> = Arc::clone(&sqlite_repo) as _;
    let review_repository: Arc<dyn ReviewRepository> = Arc::clone(&sqlite_repo) as _;
    let diary_repository: Arc<dyn DiaryRepository> = Arc::clone(&sqlite_repo) as _;
    let stats_repository: Arc<dyn StatsRepository> = Arc::clone(&sqlite_repo) as _;

    let user_repository: Arc<dyn UserRepository> =
        Arc::new(SqliteUserRepository::new(pool.clone()));
    let metadata_client: Arc<dyn MetadataClient> =
        Arc::new(MetadataClientImpl::new_omdb(omdb_api_key));
    let poster_fetcher: Arc<dyn PosterFetcherClient> =
        Arc::new(ReqwestPosterFetcher::new(PosterFetcherConfig::from_env())?);
    let poster_storage: Arc<dyn PosterStorage> =
        Arc::new(PosterStorageAdapter::from_config(storage_config));
    let auth_service: Arc<dyn AuthService> = Arc::new(JwtAuthService::new(auth_config));
    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);

    // Build a context for the poster handler. sync_poster doesn't publish events,
    // so a noop publisher here is safe and avoids a circular dependency.
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

    // Federation
    let federation_repo = Arc::new(SqliteFederationRepository::new(pool));
    let user_repo_adapter = Arc::new(DomainUserRepoAdapter(Arc::clone(&user_repository)));
    let review_handler = Arc::new(ReviewObjectHandler {
        movie_repository: Arc::clone(&movie_repository),
        diary_repository: Arc::clone(&diary_repository),
        review_store: Arc::clone(&federation_repo) as Arc<dyn activitypub::RemoteReviewRepository>,
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
    let ap_service: Arc<dyn ActivityPubPort> = concrete_ap_service;

    let poster_handler = PosterSyncHandler::new(handler_ctx, 3);
    let (event_publisher, event_worker) = create_event_channel(
        EventPublisherConfig::from_env(),
        vec![Box::new(poster_handler), Box::new(ap_event_handler)],
    );
    tokio::spawn(event_worker.run());

    let app_ctx = AppContext {
        movie_repository,
        review_repository,
        diary_repository,
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        stats_repository,
        metadata_client,
        poster_fetcher,
        poster_storage,
        event_publisher: Arc::new(event_publisher),
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
        ap_service,
    };
    Ok((state, ap_router))
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
