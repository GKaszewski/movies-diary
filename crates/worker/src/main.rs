use std::sync::Arc;

use anyhow::Context;
use application::{config::AppConfig, context::AppContext, worker::WorkerService};
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use domain::ports::{DiaryExporter, DocumentParser, EventHandler};

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("At least one database backend must be enabled. Use --features sqlite or --features postgres");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());
    let app_config = AppConfig::from_env();

    let (auth_service, password_hasher) = auth::create()?;
    let metadata_client = metadata::create()?;
    let poster_fetcher = poster_fetcher::create()?;
    let image_storage = image_storage::create()?;

    let (movie_repository, review_repository, diary_repository, stats_repository, user_repository, import_session_repository, import_profile_repository, db_pool) =
        match backend.as_str() {
            #[cfg(feature = "postgres")]
            "postgres" => {
                let (pool, m, r, d, s, u, is, ip) = postgres::wire(&database_url).await?;
                (m, r, d, s, u, is, ip, DbPool::Postgres(pool))
            }
            #[cfg(feature = "sqlite")]
            _ => {
                let (pool, m, r, d, s, u, is, ip) = sqlite::wire(&database_url).await?;
                (m, r, d, s, u, is, ip, DbPool::Sqlite(pool))
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
            match &db_pool {
                #[cfg(feature = "postgres")]
                DbPool::Postgres(pool) => postgres_event_queue::PostgresEventQueue::create_channel(pool.clone()).await?,
                #[cfg(feature = "sqlite")]
                DbPool::Sqlite(pool) => sqlite_event_queue::SqliteEventQueue::create_channel(pool.clone()).await?,
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

    // Clone what federation handler needs before ctx and app_config are consumed.
    #[cfg(feature = "federation")]
    let (fed_movie_repo, fed_review_repo, fed_diary_repo, fed_user_repo, base_url) = (
        Arc::clone(&movie_repository),
        Arc::clone(&review_repository),
        Arc::clone(&diary_repository),
        Arc::clone(&user_repository),
        app_config.base_url.clone(),
    );

    let ctx = AppContext {
        movie_repository,
        review_repository,
        diary_repository,
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        document_parser: Arc::new(ImporterDocumentParser) as Arc<dyn DocumentParser>,
        stats_repository,
        metadata_client,
        poster_fetcher,
        image_storage,
        event_publisher: event_publisher_arc,
        auth_service,
        password_hasher,
        user_repository,
        import_session_repository,
        import_profile_repository,
        config: app_config,
    };

    // Spawn periodic import session cleanup (hourly)
    {
        let cleanup_ctx = ctx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match application::use_cases::cleanup_expired_import_sessions::execute(&cleanup_ctx).await {
                    Ok(n) => tracing::info!("import session cleanup: removed {} expired sessions", n),
                    Err(e) => tracing::error!("import session cleanup failed: {:?}", e),
                }
            }
        });
    }

    let handlers: Vec<Arc<dyn EventHandler>> = {
        let poster = Arc::new(poster_sync::PosterSyncHandler::new(
            Arc::clone(&ctx.movie_repository),
            Arc::clone(&ctx.metadata_client),
            Arc::clone(&ctx.poster_fetcher),
            Arc::clone(&ctx.image_storage),
            3,
        )) as Arc<dyn EventHandler>;

        let cleanup = Arc::new(image_storage::ImageCleanupHandler::new(
            Arc::clone(&ctx.image_storage),
        )) as Arc<dyn EventHandler>;

        #[cfg(not(feature = "federation"))]
        { vec![poster, cleanup] }

        #[cfg(feature = "federation")]
        {
            let (federation_repo, _social_query, review_store) = match &db_pool {
                #[cfg(feature = "sqlite-federation")]
                DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
                #[cfg(feature = "postgres-federation")]
                DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
            };

            let ap = activitypub::wire(
                federation_repo,
                review_store,
                fed_user_repo,
                fed_movie_repo,
                fed_review_repo,
                fed_diary_repo,
                base_url,
            ).await?.event_handler;

            tracing::info!("federation event handler registered");
            vec![poster, cleanup, ap]
        }
    };

    let worker = WorkerService::new(consumer_arc, handlers);

    tracing::info!("worker started");
    worker.run().await;
    tracing::info!("worker stopped");

    Ok(())
}

enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
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

