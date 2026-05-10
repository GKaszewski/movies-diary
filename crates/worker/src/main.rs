use std::sync::Arc;
use std::str::FromStr;

use anyhow::Context;
use application::{config::AppConfig, context::AppContext, event_handlers::PosterSyncHandler, worker::WorkerService};
use auth::{Argon2PasswordHasher, AuthConfig, JwtAuthService};
use export::ExportAdapter;
use metadata::MetadataClientImpl;
use poster_fetcher::{PosterFetcherConfig, ReqwestPosterFetcher};
use poster_storage::{PosterStorageAdapter, StorageConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "sqlite")]
use sqlite::{SqliteMovieRepository, SqliteUserRepository};

#[cfg(feature = "postgres")]
use postgres::{PostgresRepository, PostgresUserRepository};

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

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());
    let auth_config = AuthConfig::from_env()?;
    let storage_config = StorageConfig::from_env()?;
    let app_config = AppConfig::from_env();

    let metadata_client: Arc<dyn MetadataClient> =
        if let Ok(tmdb_key) = std::env::var("TMDB_API_KEY") {
            Arc::new(MetadataClientImpl::new_tmdb(tmdb_key))
        } else {
            let omdb_key = std::env::var("OMDB_API_KEY")
                .context("Either TMDB_API_KEY or OMDB_API_KEY must be set")?;
            Arc::new(MetadataClientImpl::new_omdb(omdb_key))
        };
    let poster_fetcher: Arc<dyn PosterFetcherClient> =
        Arc::new(ReqwestPosterFetcher::new(PosterFetcherConfig::from_env())?);
    let poster_storage: Arc<dyn PosterStorage> =
        Arc::new(PosterStorageAdapter::from_config(storage_config));
    let auth_service: Arc<dyn AuthService> = Arc::new(JwtAuthService::new(auth_config));
    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);

    #[cfg(feature = "sqlite")]
    let mut sqlite_pool: Option<sqlx::SqlitePool> = None;
    #[cfg(feature = "postgres")]
    let mut pg_pool: Option<sqlx::PgPool> = None;

    let (movie_repository, review_repository, diary_repository, stats_repository, user_repository):
        (Arc<dyn MovieRepository>, Arc<dyn ReviewRepository>, Arc<dyn DiaryRepository>,
         Arc<dyn StatsRepository>, Arc<dyn UserRepository>) =
        match backend.as_str() {
            #[cfg(feature = "postgres")]
            "postgres" => {
                let (pool, m, r, d, s, u) = wire_postgres(&database_url).await?;
                pg_pool = Some(pool);
                (m, r, d, s, u)
            }
            #[cfg(feature = "sqlite")]
            _ => {
                let (pool, m, r, d, s, u) = wire_sqlite(&database_url).await?;
                sqlite_pool = Some(pool);
                (m, r, d, s, u)
            }
            #[cfg(not(feature = "sqlite"))]
            _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build"),
        };

    let (event_publisher_arc, consumer_arc): (
        Arc<dyn domain::ports::EventPublisher>,
        Arc<dyn domain::ports::EventConsumer>,
    ) = match EventBusBackend::from_env()? {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            match backend.as_str() {
                #[cfg(feature = "postgres")]
                "postgres" => postgres_event_queue::PostgresEventQueue::create_channel(
                    pg_pool.unwrap()
                ).await?,
                #[cfg(feature = "sqlite")]
                _ => sqlite_event_queue::SqliteEventQueue::create_channel(
                    sqlite_pool.unwrap()
                ).await?,
                #[cfg(not(feature = "sqlite"))]
                _ => anyhow::bail!("EVENT_BUS_BACKEND=db has no adapter for DATABASE_BACKEND={backend}; enable the sqlite or postgres feature"),
            }
        }
        #[cfg(feature = "nats")]
        EventBusBackend::Nats => {
            let cfg = nats::NatsConfig::from_env()
                .context("EVENT_BUS_BACKEND=nats requires NATS_URL to be set")?;
            tracing::info!("event bus: NATS ({})", cfg.url);
            nats::create_channel(cfg).await?
        }
    };

    let ctx = AppContext {
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

    let poster_handler = Arc::new(PosterSyncHandler::new(ctx, 3));
    let worker = WorkerService::new(consumer_arc, vec![poster_handler]);

    tracing::info!("worker started");
    worker.run().await;
    tracing::info!("worker stopped");

    Ok(())
}

#[derive(Clone, Copy)]
enum EventBusBackend {
    Db,
    #[cfg(feature = "nats")]
    Nats,
}

impl EventBusBackend {
    fn from_env() -> anyhow::Result<Self> {
        match std::env::var("EVENT_BUS_BACKEND")
            .unwrap_or_else(|_| "db".to_string())
            .as_str()
        {
            "db" => Ok(Self::Db),
            #[cfg(feature = "nats")]
            "nats" => Ok(Self::Nats),
            #[cfg(not(feature = "nats"))]
            "nats" => anyhow::bail!("EVENT_BUS_BACKEND=nats requires the nats feature to be compiled in"),
            other => anyhow::bail!("unknown EVENT_BUS_BACKEND={other}, expected 'db' or 'nats'"),
        }
    }
}

fn init_tracing() {
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "worker=info,application=info".to_string());
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();
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
