use std::sync::Arc;

use anyhow::Context;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::config::AppConfig;
use composition::Repositories;
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use presentation::context::AppContext;
use presentation::{openapi, routes, state::AppState};
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
    let instance = domain::value_objects::InstanceIdentity::new(app_config.base_url.clone());
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let backend = std::env::var("DATABASE_BACKEND").unwrap_or_else(|_| "sqlite".to_string());

    let (auth_service, password_hasher) = composition::factory::build_auth_adapters()?;
    let metadata_client = composition::factory::build_metadata_client()?;
    let poster_fetcher = composition::factory::build_poster_fetcher()?;
    let object_storage = composition::factory::build_object_storage()?;

    let db =
        composition::factory::build_database_adapters(&backend, &database_url, &instance).await?;
    let ap_content_repo = db.ap_content;
    let db_pool = db.db_pool;

    // Wire up event channel, federation service, and ap_router
    let event_bus = EventBusBackend::from_env()?;

    #[cfg(feature = "federation")]
    let (
        event_publisher_arc,
        ap_router,
        ap_document,
        ap_blocklist,
        social_query,
        remote_watchlist_repo,
        social_command_arc,
        follow_graph_arc,
        block_query_arc,
    ) = {
        let fed_repos = match &db_pool {
            #[cfg(feature = "postgres-federation")]
            composition::DbPool::Postgres(pool) => {
                postgres_federation::wire(pool.clone(), instance.clone())
            }
            #[cfg(feature = "sqlite-federation")]
            composition::DbPool::Sqlite(pool) => {
                sqlite_federation::wire(pool.clone(), instance.clone())
            }
            #[cfg(not(feature = "sqlite-federation"))]
            _ => anyhow::bail!(
                "DATABASE_BACKEND={backend} federation is not supported by this build"
            ),
        };

        let ep = create_event_publisher(event_bus, &db_pool).await?;

        let ap = activitypub::wire(activitypub::ActivityPubDeps {
            activity_repo: fed_repos.activity,
            follow_repo: fed_repos.follow,
            actor_repo: fed_repos.actor,
            blocklist_repo: fed_repos.blocklist,
            review_store: fed_repos.review_store,
            remote_watchlist_repo: fed_repos.remote_watchlist.clone(),
            remote_goal_repo: Arc::clone(&db.remote_goal),
            local_ap_content: Arc::clone(&ap_content_repo),
            movie_repo: Arc::clone(&db.movie_query),
            review_repo: Arc::clone(&db.review),
            diary_repo: Arc::clone(&db.diary),
            goal_repo: Arc::clone(&db.goal_query),
            stats_repo: Arc::clone(&db.stats),
            user_repo: Arc::clone(&db.user),
            federation_settings: std::sync::Arc::clone(&db.federation_settings),
            follow_command: Arc::clone(&fed_repos.follow_command),
            follow_query: Arc::clone(&fed_repos.follow_query),
            instance: instance.clone(),
            allow_registration: app_config.allow_registration,
            event_publisher: Arc::clone(&ep),
        })
        .await?;
        let ap_router = ap.router;
        let ap_document = ap.document;
        let ap_blocklist = ap.blocklist;

        let local_social: Arc<dyn domain::ports::LocalSocial> =
            Arc::new(application::social::local_service::LocalSocialService::new(
                Arc::clone(&db.user),
                Arc::clone(&db.follow_command),
                Arc::clone(&db.follow_query),
                instance.clone(),
            ));

        let composite_social = Arc::new(activitypub::CompositeSocialAdapter::new(
            local_social,
            ap.service,
            Arc::clone(&db.user),
            instance.clone(),
        ));

        (
            ep,
            ap_router,
            ap_document,
            ap_blocklist,
            fed_repos.admin_query,
            fed_repos.remote_watchlist,
            composite_social.clone() as Arc<dyn domain::ports::SocialCommand>,
            composite_social.clone() as Arc<dyn domain::ports::FollowGraphQuery>,
            composite_social as Arc<dyn domain::ports::BlockQuery>,
        )
    };

    #[cfg(not(feature = "federation"))]
    let event_publisher_arc = create_event_publisher(event_bus, &db_pool).await?;
    #[cfg(not(feature = "federation"))]
    let ap_router = axum::Router::new();
    #[cfg(not(feature = "federation"))]
    let (social_command_arc, follow_graph_arc, block_query_arc) = {
        let local = Arc::new(application::social::local_service::LocalSocialService::new(
            Arc::clone(&db.user),
            Arc::clone(&db.follow_command),
            Arc::clone(&db.follow_query),
            instance.clone(),
        ));
        (
            Arc::clone(&local) as Arc<dyn domain::ports::SocialCommand>,
            Arc::clone(&local) as Arc<dyn domain::ports::FollowGraphQuery>,
            local as Arc<dyn domain::ports::BlockQuery>,
        )
    };

    let review_logger = Arc::new(application::diary::review_logger::DefaultReviewLogger::new(
        Arc::clone(&db.movie_command),
        Arc::clone(&db.movie_query),
        Arc::clone(&db.review),
        Arc::clone(&db.watchlist),
        Arc::clone(&metadata_client),
        Arc::clone(&event_publisher_arc),
    ));

    let repos = Repositories {
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
        follow_graph: follow_graph_arc,
        block_query: block_query_arc,
        #[cfg(feature = "federation")]
        federation_admin: social_query.clone(),
        // The empty list this noop returns is an invariant, not an approximation:
        // federation-off, `LocalSocialService::follow_resolved` hard-errors on any
        // non-Local target and block/unblock always error, so no remote follow can
        // ever be persisted through this wiring. (Stale rows from an instance
        // previously built federation-ON are the one exception — display staleness
        // only; see ADR-0009.)
        #[cfg(not(feature = "federation"))]
        federation_admin: Arc::new(domain::ports::noop::NoopFederationAdminQuery),
        wrapup_stats: db.wrapup_stats,
        wrapup_repo: db.wrapup_repo,
        insights: db.insights,
        goal_command: db.goal_command,
        goal_query: db.goal_query,
        user_settings: db.user_settings,
        remote_goal: db.remote_goal,
        refresh_session: db.refresh_session,
        #[cfg(feature = "federation")]
        federated_profile: Some({
            match &db_pool {
                #[cfg(feature = "sqlite-federation")]
                composition::DbPool::Sqlite(pool) => {
                    sqlite_social::create_federated_profile_query(pool.clone(), instance.clone())
                }
                #[cfg(feature = "postgres-federation")]
                composition::DbPool::Postgres(pool) => {
                    postgres_social::create_federated_profile_query(pool.clone(), instance.clone())
                }
                #[cfg(not(feature = "sqlite-federation"))]
                _ => unreachable!(),
            }
        }),
        #[cfg(not(feature = "federation"))]
        federated_profile: None,
    };

    let services = application::Services {
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
    };

    let deps = Arc::new(composition::build_deps(
        &repos,
        &services,
        &app_config,
        &instance,
    ));

    let app_ctx = AppContext {
        deps,
        services,
        config: app_config,
        instance,
        #[cfg(feature = "federation")]
        ap_document,
        #[cfg(feature = "federation")]
        ap_blocklist,
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
    db_pool: &composition::DbPool,
) -> anyhow::Result<Arc<dyn EventPublisher>> {
    match event_bus {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            Ok(match db_pool {
                #[cfg(feature = "postgres")]
                composition::DbPool::Postgres(pool) => {
                    postgres_event_queue::PostgresEventQueue::create_publisher(pool.clone()).await?
                }
                #[cfg(feature = "sqlite")]
                composition::DbPool::Sqlite(pool) => {
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
        .with(log_filter("server=debug,tower_http=debug"))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// `RUST_LOG` names the crates to trace in detail; anything it doesn't name
/// still reports warnings and errors. Without a baseline, a filter like
/// `presentation=debug` silently drops adapter errors (e.g. the sqlx detail
/// behind "Database operation failed").
fn log_filter(default: &str) -> tracing_subscriber::EnvFilter {
    let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| default.to_string());
    let has_baseline = directives
        .split(',')
        .any(|d| !d.trim().is_empty() && !d.contains('='));
    if has_baseline {
        tracing_subscriber::EnvFilter::new(directives)
    } else {
        tracing_subscriber::EnvFilter::new(format!("warn,{directives}"))
    }
}
