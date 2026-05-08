use std::sync::Arc;

use anyhow::Context;
use event_publisher::{EventPublisherConfig, NoopEventPublisher, create_event_channel};
use presentation::event_handlers::PosterSyncHandler;
use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{config::AppConfig, context::AppContext};
use auth::{AuthConfig, Argon2PasswordHasher, JwtAuthService};
use metadata::MetadataClientImpl;
use poster_fetcher::{PosterFetcherConfig, ReqwestPosterFetcher};
use poster_storage::{PosterStorageAdapter, StorageConfig};
use activitypub::ActivityPubService;
use sqlite::{SqliteFederationRepository, SqliteMovieRepository, SqliteUserRepository};
use rss::RssAdapter;
use template_askama::AskamaHtmlRenderer;

use presentation::{routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let state = wire_dependencies()
        .await
        .context("Failed to wire dependencies")?;

    let app = routes::build_router(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<AppState> {
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

    let movie_repo = SqliteMovieRepository::new(pool.clone());
    movie_repo
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Database migration failed")?;

    use domain::ports::{
        AuthService, MetadataClient, MovieRepository, PasswordHasher,
        PosterFetcherClient, PosterStorage, UserRepository,
    };
    let repository: Arc<dyn MovieRepository> = Arc::new(movie_repo);
    let user_repository: Arc<dyn UserRepository> = Arc::new(SqliteUserRepository::new(pool.clone()));
    let metadata_client: Arc<dyn MetadataClient> = Arc::new(MetadataClientImpl::new_omdb(omdb_api_key));
    let poster_fetcher: Arc<dyn PosterFetcherClient> = Arc::new(ReqwestPosterFetcher::new(PosterFetcherConfig::from_env())?);
    let poster_storage: Arc<dyn PosterStorage> = Arc::new(PosterStorageAdapter::from_config(storage_config)?);
    let auth_service: Arc<dyn AuthService> = Arc::new(JwtAuthService::new(auth_config));
    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);

    // Build a context for the poster handler. sync_poster doesn't publish events,
    // so a noop publisher here is safe and avoids a circular dependency.
    let handler_ctx = AppContext {
        repository: Arc::clone(&repository),
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
    let ap_service = Arc::new(
        ActivityPubService::new(
            federation_repo,
            Arc::clone(&user_repository),
            app_config.base_url.clone(),
            cfg!(debug_assertions),
        )
        .await?,
    );
    let ap_event_handler = ap_service.event_handler();

    let poster_handler = PosterSyncHandler::new(handler_ctx, 3);
    let (event_publisher, event_worker) = create_event_channel(
        EventPublisherConfig::from_env(),
        vec![Box::new(poster_handler), Box::new(ap_event_handler)],
    );
    tokio::spawn(event_worker.run());

    let app_ctx = AppContext {
        repository,
        metadata_client,
        poster_fetcher,
        poster_storage,
        event_publisher: Arc::new(event_publisher),
        auth_service,
        password_hasher,
        user_repository,
        config: app_config,
    };

    Ok(AppState {
        app_ctx,
        html_renderer: Arc::new(AskamaHtmlRenderer::new()),
        rss_renderer: Arc::new(RssAdapter::new(
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
        )),
        ap_service,
    })
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
