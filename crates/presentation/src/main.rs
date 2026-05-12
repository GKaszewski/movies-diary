use std::sync::Arc;

use anyhow::Context;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{config::AppConfig, context::AppContext};
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use rss::RssAdapter;
use template_askama::AskamaHtmlRenderer;

use presentation::{openapi, routes, state::AppState};

use domain::ports::{DiaryExporter, DocumentParser, EventPublisher, ImportProfileRepository, ImportSessionRepository};

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("At least one database backend must be enabled. Use --features sqlite or --features postgres");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let (state, ap_router) = wire_dependencies()
        .await
        .context("Failed to wire dependencies")?;

    let app = openapi::serve(routes::build_router(state, ap_router));

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<(AppState, axum::Router)> {
    let app_config = AppConfig::from_env();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());

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
            _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build (sqlite feature is not enabled)"),
        };

    // Wire up event channel, federation service, and ap_router
    let event_bus = EventBusBackend::from_env()?;

    #[cfg(feature = "federation")]
    let (event_publisher_arc, ap_router, ap_service, social_query) = {
        let (federation_repo, social_query_arc, review_store) = match &db_pool {
            #[cfg(feature = "postgres-federation")]
            DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
            #[cfg(feature = "sqlite-federation")]
            DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
            #[cfg(not(feature = "sqlite-federation"))]
            _ => anyhow::bail!("DATABASE_BACKEND={backend} federation is not supported by this build"),
        };

        let ap = activitypub::wire(
            federation_repo,
            review_store,
            Arc::clone(&user_repository),
            Arc::clone(&movie_repository),
            Arc::clone(&review_repository),
            Arc::clone(&diary_repository),
            app_config.base_url.clone(),
            app_config.allow_registration,
        ).await?;
        let ap_router = ap.router;
        let ap_service_arc = ap.service;

        let ep: Arc<dyn EventPublisher> = match event_bus {
            EventBusBackend::Db => {
                tracing::info!("event bus: DB queue");
                match &db_pool {
                    #[cfg(feature = "postgres")]
                    DbPool::Postgres(pool) => postgres_event_queue::PostgresEventQueue::create_publisher(
                        pool.clone()
                    ).await?,
                    #[cfg(feature = "sqlite")]
                    DbPool::Sqlite(pool) => sqlite_event_queue::SqliteEventQueue::create_publisher(
                        pool.clone()
                    ).await?,
                }
            }
            #[cfg(feature = "nats")]
            EventBusBackend::Nats => {
                let cfg = nats::NatsConfig::from_env()
                    .context("EVENT_BUS_BACKEND=nats requires NATS_URL to be set")?;
                tracing::info!("event bus: NATS ({})", cfg.url);
                nats::create_publisher(cfg).await?
            }
        };
        (ep, ap_router, ap_service_arc, social_query_arc)
    };

    #[cfg(not(feature = "federation"))]
    let event_publisher_arc: Arc<dyn EventPublisher> = match event_bus {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            match backend.as_str() {
                #[cfg(feature = "postgres")]
                "postgres" => postgres_event_queue::PostgresEventQueue::create_publisher(
                    pg_pool.as_ref().unwrap().clone()
                ).await?,
                #[cfg(feature = "sqlite")]
                _ => sqlite_event_queue::SqliteEventQueue::create_publisher(
                    sqlite_pool.as_ref().unwrap().clone()
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
            nats::create_publisher(cfg).await?
        }
    };
    #[cfg(not(feature = "federation"))]
    let ap_router = axum::Router::new();

    let app_ctx = AppContext {
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
        import_session_repository: import_session_repository as Arc<dyn ImportSessionRepository>,
        import_profile_repository: import_profile_repository as Arc<dyn ImportProfileRepository>,
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
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "presentation=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
