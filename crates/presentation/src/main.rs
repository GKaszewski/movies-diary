use std::sync::Arc;

use anyhow::Context;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::config::AppConfig;
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use presentation::context::{AppContext, Repositories, Services};
use presentation::{factory, openapi, routes, state::AppState};
use rss::RssAdapter;

use domain::ports::{DiaryExporter, DocumentParser, EventPublisher};
use infra_wiring::EventBusBackend;

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
    let (event_publisher_arc, ap_router, ap_service, social_query, remote_watchlist_repo, social_command_arc, social_query_unified_arc) = {
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

        let ep = create_event_publisher(event_bus, &db_pool).await?;

        let ap = activitypub::wire(activitypub::ActivityPubDeps {
            activity_repo,
            follow_repo,
            actor_repo,
            blocklist_repo,
            review_store,
            remote_watchlist_repo: remote_watchlist_repo.clone(),
            remote_goal_repo: Arc::clone(&db.remote_goal),
            local_ap_content: Arc::clone(&ap_content_repo),
            movie_repo: Arc::clone(&db.movie_query),
            review_repo: Arc::clone(&db.review),
            diary_repo: Arc::clone(&db.diary),
            goal_repo: Arc::clone(&db.goal_query),
            stats_repo: Arc::clone(&db.stats),
            user_repo: Arc::clone(&db.user),
            federation_settings: std::sync::Arc::clone(&db.federation_settings),
            base_url: app_config.base_url.clone(),
            allow_registration: app_config.allow_registration,
            event_publisher: Arc::clone(&ep),
        })
        .await?;
        let ap_router = ap.router;
        let ap_service_arc = ap.service;

        let composite_social = Arc::new(activitypub::CompositeSocialAdapter::new(
            Arc::clone(&ap_service_arc),
            Arc::clone(&db.user),
            app_config.base_url.clone(),
        ));

        (
            ep,
            ap_router,
            ap_service_arc,
            social_query_arc,
            remote_watchlist_repo,
            composite_social.clone() as Arc<dyn domain::ports::SocialCommand>,
            composite_social as Arc<dyn domain::ports::SocialQuery>,
        )
    };

    #[cfg(not(feature = "federation"))]
    let event_publisher_arc = create_event_publisher(event_bus, &db_pool).await?;
    #[cfg(not(feature = "federation"))]
    let ap_router = axum::Router::new();
    #[cfg(not(feature = "federation"))]
    let social_command_arc: Arc<dyn domain::ports::SocialCommand> =
        Arc::new(domain::ports::noop::NoopSocialCommand);
    #[cfg(not(feature = "federation"))]
    let social_query_unified_arc: Arc<dyn domain::ports::SocialQuery> =
        Arc::new(domain::ports::noop::NoopSocialQuery);

    let review_logger = Arc::new(application::diary::review_logger::DefaultReviewLogger::new(
        Arc::clone(&db.movie_command),
        Arc::clone(&db.movie_query),
        Arc::clone(&db.review),
        Arc::clone(&db.watchlist),
        Arc::clone(&metadata_client),
        Arc::clone(&event_publisher_arc),
    ));

    let app_ctx = AppContext {
        repos: Repositories {
            movie_command: db.movie_command,
            movie_query: db.movie_query,
            review: db.review,
            diary: db.diary,
            stats: db.stats,
            user: db.user,
            import_session: db.import_session,
            import_profile: db.import_profile,
            movie_profile: db.movie_profile,
            watchlist: db.watchlist,
            watch_event_command: db.watch_event_command,
            watch_event_query: db.watch_event_query,
            webhook_token: db.webhook_token,
            person_command: db.person_command,
            person_query: db.person_query,
            search_port: db.search_port,
            search_command: db.search_command,
            profile_fields: db.profile_fields,
            #[cfg(feature = "federation")]
            remote_watchlist: remote_watchlist_repo,
            #[cfg(not(feature = "federation"))]
            remote_watchlist: Arc::new(domain::ports::noop::NoopRemoteWatchlistRepository),
            social_command: social_command_arc,
            social_query_unified: social_query_unified_arc,
            #[cfg(feature = "federation")]
            federation_admin: social_query.clone(),
            #[cfg(not(feature = "federation"))]
            federation_admin: Arc::new(domain::ports::noop::NoopFederationAdminQuery),
            wrapup_stats: db.wrapup_stats,
            wrapup_repo: db.wrapup_repo,
            goal_command: db.goal_command,
            goal_query: db.goal_query,
            user_settings: db.user_settings,
            remote_goal: db.remote_goal,
            refresh_session: db.refresh_session,
            #[cfg(feature = "federation")]
            federated_profile: Some({
                match &db_pool {
                    #[cfg(feature = "sqlite-federation")]
                    factory::DbPool::Sqlite(pool) => {
                        sqlite_federation::create_federated_profile_query(pool.clone())
                    }
                    #[cfg(feature = "postgres-federation")]
                    factory::DbPool::Postgres(pool) => {
                        postgres_federation::create_federated_profile_query(pool.clone())
                    }
                    #[cfg(not(feature = "sqlite-federation"))]
                    _ => unreachable!(),
                }
            }),
            #[cfg(not(feature = "federation"))]
            federated_profile: None,
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
            #[cfg(feature = "federation")]
            ap_service,
        },
        config: app_config,
    };

    let state = AppState {
        app_ctx,
        rss_renderer: Arc::new(RssAdapter::new(
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
        )),
    };
    Ok((state, ap_router))
}

async fn create_event_publisher(
    event_bus: EventBusBackend,
    db_pool: &factory::DbPool,
) -> anyhow::Result<Arc<dyn EventPublisher>> {
    match event_bus {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            Ok(match db_pool {
                #[cfg(feature = "postgres")]
                factory::DbPool::Postgres(pool) => {
                    postgres_event_queue::PostgresEventQueue::create_publisher(pool.clone()).await?
                }
                #[cfg(feature = "sqlite")]
                factory::DbPool::Sqlite(pool) => {
                    sqlite_event_queue::SqliteEventQueue::create_publisher(pool.clone()).await?
                }
            })
        }
        #[cfg(feature = "nats")]
        EventBusBackend::Nats => {
            let cfg = nats::NatsConfig::from_env()
                .context("EVENT_BUS_BACKEND=nats requires NATS_URL to be set")?;
            tracing::info!("event bus: NATS ({})", cfg.url);
            Ok(nats::create_publisher(cfg).await?)
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
