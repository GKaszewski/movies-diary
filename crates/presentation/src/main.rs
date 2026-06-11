use std::sync::Arc;

use anyhow::Context;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{
    config::AppConfig,
    context::{AppContext, Repositories, Services},
};
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use presentation::{factory, openapi, routes, state::AppState};
use rss::RssAdapter;

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
    let object_storage = factory::build_object_storage()?;

    let db = factory::build_database_adapters(&backend, &database_url).await?;
    let ap_content_repo = db.ap_content;
    let db_pool = db.db_pool;

    // Wire up event channel, federation service, and ap_router
    let event_bus = EventBusBackend::from_env()?;

    #[cfg(feature = "federation")]
    let (event_publisher_arc, ap_router, ap_service, social_query, remote_watchlist_repo) = {
        let (
            activity_repo,
            follow_repo,
            actor_repo,
            blocklist_repo,
            social_query_arc,
            review_store,
            remote_watchlist_repo,
        ) = match &db_pool {
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

        let ap = activitypub::wire(activitypub::ActivityPubDeps {
            activity_repo,
            follow_repo,
            actor_repo,
            blocklist_repo,
            review_store,
            remote_watchlist_repo: remote_watchlist_repo.clone(),
            remote_goal_repo: Arc::clone(&db.remote_goal),
            local_ap_content: Arc::clone(&ap_content_repo),
            user_repo: Arc::clone(&db.user),
            base_url: app_config.base_url.clone(),
            allow_registration: app_config.allow_registration,
            event_publisher: Arc::clone(&ep),
        })
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

    let review_logger = Arc::new(application::diary::review_logger::DefaultReviewLogger::new(
        Arc::clone(&db.movie),
        Arc::clone(&db.review),
        Arc::clone(&db.watchlist),
        Arc::clone(&metadata_client),
        Arc::clone(&event_publisher_arc),
    ));

    let app_ctx = AppContext {
        repos: Repositories {
            movie: db.movie,
            review: db.review,
            diary: db.diary,
            stats: db.stats,
            user: db.user,
            import_session: db.import_session,
            import_profile: db.import_profile,
            movie_profile: db.movie_profile,
            watchlist: db.watchlist,
            watch_event: db.watch_event,
            webhook_token: db.webhook_token,
            person_command: db.person_command,
            person_query: db.person_query,
            search_port: db.search_port,
            search_command: db.search_command,
            profile_fields: db.profile_fields,
            #[cfg(feature = "federation")]
            remote_watchlist: remote_watchlist_repo,
            #[cfg(not(feature = "federation"))]
            remote_watchlist: Arc::new(domain::testing::NoopRemoteWatchlistRepository),
            #[cfg(feature = "federation")]
            social_query: social_query.clone(),
            #[cfg(not(feature = "federation"))]
            social_query: Arc::new(domain::testing::NoopSocialQueryPort),
            wrapup_stats: db.wrapup_stats,
            wrapup_repo: db.wrapup_repo,
            goal: db.goal,
            user_settings: db.user_settings,
            remote_goal: db.remote_goal,
        },
        services: Services {
            auth: auth_service,
            password_hasher,
            metadata: metadata_client,
            poster_fetcher,
            object_storage,
            event_publisher: event_publisher_arc,
            diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
            document_parser: Arc::new(ImporterDocumentParser) as Arc<dyn DocumentParser>,
            review_logger,
            person_enrichment: None,
        },
        config: app_config,
    };

    let state = AppState {
        app_ctx,
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
