use std::sync::Arc;

use anyhow::Context;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{config::AppConfig, context::AppContext};
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use rss::RssAdapter;
use template_askama::AskamaHtmlRenderer;

use presentation::{factory, openapi, routes, state::AppState};

use domain::ports::{DiaryExporter, DocumentParser, EventPublisher};

#[cfg(feature = "postgres")]
use postgres_search;

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!(
    "At least one database backend must be enabled. Use --features sqlite or --features postgres"
);

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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn wire_dependencies() -> anyhow::Result<(AppState, axum::Router)> {
    let app_config = AppConfig::from_env();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());

    let (auth_service, password_hasher) = factory::build_auth_adapters()?;
    let metadata_client = factory::build_metadata_client()?;
    let poster_fetcher = factory::build_poster_fetcher()?;
    let image_storage = factory::build_image_storage()?;

    let db = factory::build_database_adapters(&backend, &database_url).await?;

    let movie_repository = db.movie_repo;
    let review_repository = db.review_repo;
    let diary_repository = db.diary_repo;
    let stats_repository = db.stats_repo;
    let user_repository = db.user_repo;
    let import_session_repository = db.import_session_repo;
    let import_profile_repository = db.import_profile_repo;
    let movie_profile_repository = db.movie_profile_repo;
    let watchlist_repository = db.watchlist_repo;
    let ap_content_repo = db.ap_content_repo;
    let person_command = db.person_command;
    let person_query = db.person_query;
    let search_port = db.search_port;
    let search_command = db.search_command;
    let profile_fields_repo = db.profile_fields_repo;
    let db_pool = db.db_pool;

    // Wire up event channel, federation service, and ap_router
    let event_bus = EventBusBackend::from_env()?;

    #[cfg(feature = "federation")]
    let (event_publisher_arc, ap_router, ap_service, social_query, remote_watchlist_repo) = {
        let (federation_repo, social_query_arc, review_store, remote_watchlist_repo) =
            match &db_pool {
                #[cfg(feature = "postgres-federation")]
                factory::DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
                #[cfg(feature = "sqlite-federation")]
                factory::DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
                #[cfg(not(feature = "sqlite-federation"))]
                _ => anyhow::bail!(
                    "DATABASE_BACKEND={backend} federation is not supported by this build"
                ),
            };

        let ep: Arc<dyn EventPublisher> = match event_bus {
            EventBusBackend::Db => {
                tracing::info!("event bus: DB queue");
                match &db_pool {
                    #[cfg(feature = "postgres")]
                    factory::DbPool::Postgres(pool) => {
                        postgres_event_queue::PostgresEventQueue::create_publisher(pool.clone())
                            .await?
                    }
                    #[cfg(feature = "sqlite")]
                    factory::DbPool::Sqlite(pool) => {
                        sqlite_event_queue::SqliteEventQueue::create_publisher(pool.clone()).await?
                    }
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

        let ap = activitypub::wire(
            federation_repo,
            review_store,
            remote_watchlist_repo.clone(),
            Arc::clone(&ap_content_repo),
            Arc::clone(&user_repository),
            app_config.base_url.clone(),
            app_config.allow_registration,
            Arc::clone(&ep),
        )
        .await?;
        let ap_router = ap.router;
        let ap_service_arc = ap.service;

        (
            ep,
            ap_router,
            ap_service_arc,
            social_query_arc,
            remote_watchlist_repo,
        )
    };

    #[cfg(not(feature = "federation"))]
    let event_publisher_arc: Arc<dyn EventPublisher> = match event_bus {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            match &db_pool {
                #[cfg(feature = "postgres")]
                factory::DbPool::Postgres(pool) => {
                    postgres_event_queue::PostgresEventQueue::create_publisher(pool.clone()).await?
                }
                #[cfg(feature = "sqlite")]
                factory::DbPool::Sqlite(pool) => {
                    sqlite_event_queue::SqliteEventQueue::create_publisher(pool.clone()).await?
                }
                #[cfg(not(feature = "sqlite"))]
                _ => anyhow::bail!(
                    "EVENT_BUS_BACKEND=db has no adapter for DATABASE_BACKEND={backend}; enable the sqlite or postgres feature"
                ),
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
        import_session_repository,
        import_profile_repository,
        movie_profile_repository,
        watchlist_repository,
        profile_fields_repository: profile_fields_repo,
        #[cfg(feature = "federation")]
        remote_watchlist_repository: remote_watchlist_repo,
        person_command,
        person_query,
        search_port,
        search_command,
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
            "nats" => {
                anyhow::bail!("EVENT_BUS_BACKEND=nats requires the nats feature to be compiled in")
            }
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
