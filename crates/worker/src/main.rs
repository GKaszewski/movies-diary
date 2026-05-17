mod db;
mod event_bus;
mod follow_backfill_handler;

use std::sync::Arc;

use anyhow::Context;
use application::{
    MovieDiscoveryIndexer, SearchCleanupHandler, config::AppConfig, context::AppContext,
    worker::WorkerService,
};
use export::ExportAdapter;
use importer::ImporterDocumentParser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use domain::ports::{DiaryExporter, DocumentParser, EventHandler, PeriodicJob};

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!(
    "At least one database backend must be enabled. Use --features sqlite or --features postgres"
);

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

    let (repos, db_pool) = db::connect(&database_url, &backend).await?;
    let (event_publisher_arc, consumer_arc) = event_bus::create(&db_pool).await?;

    let image_ref_command = Arc::clone(&repos.image_ref_command);
    let image_ref_query = Arc::clone(&repos.image_ref_query);
    let person_command = Arc::clone(&repos.person_command);
    let person_query = Arc::clone(&repos.person_query);
    let search_command = Arc::clone(&repos.search_command);
    let search_port = Arc::clone(&repos.search_port);
    let profile_fields_repo = Arc::clone(&repos.profile_fields);

    // Clone refs federation handler needs before ctx consumes them.
    #[cfg(feature = "federation")]
    let (
        fed_movie_repo,
        fed_review_repo,
        fed_diary_repo,
        fed_user_repo,
        base_url,
        allow_registration,
    ) = (
        Arc::clone(&repos.movie),
        Arc::clone(&repos.review),
        Arc::clone(&repos.diary),
        Arc::clone(&repos.user),
        app_config.base_url.clone(),
        app_config.allow_registration,
    );
    // Wire federation repos early to get remote_watchlist_repo for AppContext.
    #[cfg(feature = "federation")]
    let (fed_federation_repo, _fed_social_query, fed_review_store, fed_remote_watchlist_repo) =
        match &db_pool {
            #[cfg(feature = "sqlite-federation")]
            db::DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
            #[cfg(feature = "postgres-federation")]
            db::DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
        };

    let ctx = AppContext {
        movie_repository: repos.movie,
        review_repository: repos.review,
        diary_repository: repos.diary,
        diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
        document_parser: Arc::new(ImporterDocumentParser) as Arc<dyn DocumentParser>,
        stats_repository: repos.stats,
        metadata_client,
        poster_fetcher,
        image_storage,
        event_publisher: event_publisher_arc,
        auth_service,
        password_hasher,
        user_repository: repos.user,
        import_session_repository: repos.import_session,
        import_profile_repository: repos.import_profile,
        movie_profile_repository: repos.movie_profile,
        watchlist_repository: repos.watchlist,
        profile_fields_repository: Arc::clone(&profile_fields_repo),
        #[cfg(feature = "federation")]
        remote_watchlist_repository: fed_remote_watchlist_repo.clone(),
        person_command: Arc::clone(&person_command),
        person_query: Arc::clone(&person_query),
        search_port: Arc::clone(&search_port),
        search_command: Arc::clone(&search_command),
        config: app_config,
    };

    // ── Enrichment ────────────────────────────────────────────────────────────
    // Both the event handler and the staleness job are gated on TMDB_API_KEY.
    // Without a key, no MovieEnrichmentRequested events are produced or handled.

    let (enrichment_handler, enrichment_job): (
        Option<Arc<dyn EventHandler>>,
        Option<Arc<dyn PeriodicJob>>,
    ) = match tmdb_enrichment::TmdbEnrichmentClient::from_env() {
        Ok(client) => {
            tracing::info!("TMDb enrichment enabled");
            let handler = Arc::new(tmdb_enrichment::EnrichmentHandler {
                enrichment_client: Arc::new(client),
                movie_repository: Arc::clone(&ctx.movie_repository),
                profile_repo: Arc::clone(&ctx.movie_profile_repository),
                person_command: Arc::clone(&ctx.person_command),
                search_command: Arc::clone(&ctx.search_command),
            }) as Arc<dyn EventHandler>;
            let job = Arc::new(application::jobs::EnrichmentStalenessJob::new(ctx.clone()))
                as Arc<dyn PeriodicJob>;
            (Some(handler), Some(job))
        }
        Err(e) => {
            tracing::warn!("TMDb enrichment disabled: {e}");
            (None, None)
        }
    };

    // ── Image conversion ──────────────────────────────────────────────────────

    let conversion = image_converter::build(
        Arc::clone(&ctx.image_storage),
        image_ref_command,
        image_ref_query,
        Arc::clone(&ctx.event_publisher),
    )?;

    // ── Periodic jobs ─────────────────────────────────────────────────────────

    let mut periodic_jobs: Vec<Arc<dyn PeriodicJob>> = vec![Arc::new(
        application::jobs::ImportSessionCleanupJob::new(ctx.clone()),
    )];
    if let Some(job) = enrichment_job {
        periodic_jobs.push(job);
    }
    if let Some((_, ref conv_job)) = conversion {
        periodic_jobs.push(Arc::clone(conv_job));
    }

    for job in periodic_jobs {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(job.interval());
            loop {
                tick.tick().await;
                if let Err(e) = job.run().await {
                    tracing::error!("periodic job failed: {e}");
                }
            }
        });
    }

    // ── Event handlers ────────────────────────────────────────────────────────

    let handlers: Vec<Arc<dyn EventHandler>> = {
        let poster = Arc::new(poster_sync::PosterSyncHandler::new(
            Arc::clone(&ctx.movie_repository),
            Arc::clone(&ctx.metadata_client),
            Arc::clone(&ctx.poster_fetcher),
            Arc::clone(&ctx.image_storage),
            Arc::clone(&ctx.event_publisher),
            3,
        )) as Arc<dyn EventHandler>;

        let cleanup = Arc::new(image_storage::ImageCleanupHandler::new(Arc::clone(
            &ctx.image_storage,
        ))) as Arc<dyn EventHandler>;

        #[cfg(not(feature = "federation"))]
        {
            let search_cleanup = Arc::new(SearchCleanupHandler::new(
                Arc::clone(&ctx.search_command),
                Arc::clone(&ctx.person_query),
            )) as Arc<dyn EventHandler>;
            let discovery_indexer = Arc::new(MovieDiscoveryIndexer::new(
                Arc::clone(&ctx.movie_repository),
                Arc::clone(&ctx.search_command),
            )) as Arc<dyn EventHandler>;
            let mut h = vec![poster, cleanup, search_cleanup, discovery_indexer];
            if let Some(e) = enrichment_handler {
                h.push(e);
            }
            if let Some((ref conv_handler, _)) = conversion {
                h.push(Arc::clone(conv_handler));
            }
            h
        }

        #[cfg(feature = "federation")]
        {
            let ap_wire = activitypub::wire(
                fed_federation_repo,
                fed_review_store,
                fed_remote_watchlist_repo,
                fed_user_repo,
                fed_movie_repo,
                fed_review_repo,
                fed_diary_repo,
                base_url,
                allow_registration,
                Arc::clone(&ctx.event_publisher),
            )
            .await?;

            let ap_event_handler = ap_wire.event_handler;
            let backfill = Arc::new(follow_backfill_handler::FollowBackfillHandler {
                ap_service: ap_wire.service,
            }) as Arc<dyn EventHandler>;

            let search_cleanup = Arc::new(SearchCleanupHandler::new(
                Arc::clone(&ctx.search_command),
                Arc::clone(&ctx.person_query),
            )) as Arc<dyn EventHandler>;
            let discovery_indexer = Arc::new(MovieDiscoveryIndexer::new(
                Arc::clone(&ctx.movie_repository),
                Arc::clone(&ctx.search_command),
            )) as Arc<dyn EventHandler>;
            tracing::info!("federation event handler registered");
            let mut h = vec![
                poster,
                cleanup,
                ap_event_handler,
                backfill,
                search_cleanup,
                discovery_indexer,
            ];
            if let Some(e) = enrichment_handler {
                h.push(e);
            }
            if let Some((ref conv_handler, _)) = conversion {
                h.push(Arc::clone(conv_handler));
            }
            h
        }
    };

    // ── Run ───────────────────────────────────────────────────────────────────

    let worker = WorkerService::new(consumer_arc, handlers);
    tracing::info!("worker started");
    worker.run().await;
    tracing::info!("worker stopped");

    Ok(())
}

fn init_tracing() {
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "worker=info,application=info,k_ap=info".to_string());
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
