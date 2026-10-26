use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use application::config::AppSettings;
use domain::ports::{DiaryExporter, DocumentParser, EventPublisher};
use domain::value_objects::InstanceIdentity;
use export::ExportAdapter;
use http_axum::context::AppContext;
use http_axum::{openapi, router, state::AppState};
use importer::ImporterDocumentParser;
use rss_adapter::RssAdapter;

use crate::database::{self, DatabaseOutput};
use crate::{Repositories, event_bus, federation};

pub struct ServerApp {
    pub router: axum::Router,
    pub addr: SocketAddr,
}

pub async fn build_server(config: &config::AppConfig) -> anyhow::Result<ServerApp> {
    database::warn_if_database_backend_set();
    let instance = InstanceIdentity::new(config.server.base_url.clone());
    let db = database::build_database_adapters(&config.database.url, &instance).await?;
    let router = assemble(config, db, instance)
        .await
        .context("Failed to wire dependencies")?;

    let bind = format!("{}:{}", config.server.host, config.server.port);
    let addr = tokio::net::lookup_host(&bind)
        .await
        .with_context(|| format!("Invalid HOST/PORT {bind}"))?
        .next()
        .with_context(|| format!("HOST/PORT {bind} resolved to no address"))?;
    Ok(ServerApp { router, addr })
}

pub async fn build_server_on(
    config: &config::AppConfig,
    pool: sqlx::SqlitePool,
) -> anyhow::Result<axum::Router> {
    let instance = InstanceIdentity::new(config.server.base_url.clone());
    let db = database::build_database_adapters_on(pool, &instance).await?;
    assemble(config, db, instance).await
}

async fn assemble(
    config: &config::AppConfig,
    db: DatabaseOutput,
    instance: InstanceIdentity,
) -> anyhow::Result<axum::Router> {
    crate::tls::install_crypto_provider();
    let app_config = AppSettings::from(config);

    let (auth_service, password_hasher) = database::build_auth_adapters(&config.auth)?;
    let metadata_client = database::build_metadata_client(&config.metadata)?;
    let poster_fetcher = database::build_poster_fetcher(&config.metadata)?;
    let object_storage = database::build_object_storage(&config.storage)?;

    let event_publisher: Arc<dyn EventPublisher> =
        event_bus::create_publisher(&db.db_pool, config).await?;

    let social = federation::wire_server_social(&db, &instance, config, &event_publisher).await?;

    let review_logger = Arc::new(application::diary::review_logger::DefaultReviewLogger::new(
        Arc::clone(&db.movie_command),
        Arc::clone(&db.movie_query),
        Arc::clone(&db.review),
        Arc::clone(&db.watchlist),
        Arc::clone(&metadata_client),
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
        remote_watchlist: social.remote_watchlist,
        social_command: social.social_command,
        follow_graph: social.follow_graph,
        block_query: social.block_query,
        federation_admin: social.federation_admin,
        wrapup_stats: db.wrapup_stats,
        wrapup_repo: db.wrapup_repo,
        insights: db.insights,
        goal_command: db.goal_command,
        goal_query: db.goal_query,
        user_settings: db.user_settings,
        remote_goal: db.remote_goal,
        refresh_session: db.refresh_session,
        federated_profile: social.federated_profile,
    };

    let services = application::Services {
        auth: auth_service,
        password_hasher,
        metadata: metadata_client,
        poster_fetcher,
        object_storage,
        event_publisher,
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        document_parser: Arc::new(ImporterDocumentParser) as Arc<dyn DocumentParser>,
        review_logger,
        person_enrichment: None,
    };

    let deps = Arc::new(crate::build_deps(&repos, &services, &app_config, &instance));

    let app_ctx = AppContext {
        deps,
        services,
        config: app_config,
        instance,
        #[cfg(feature = "federation")]
        ap_document: social.ap_document,
        #[cfg(feature = "federation")]
        ap_blocklist: social.ap_blocklist,
    };

    let state = AppState {
        app_ctx,
        rss_renderer: Arc::new(RssAdapter::new(config.server.base_url.clone())),
    };
    Ok(openapi::serve(router::build_router(
        state,
        social.ap_router,
    )))
}
