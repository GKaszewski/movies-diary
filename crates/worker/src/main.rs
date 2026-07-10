mod db;
mod event_bus;
mod follow_backfill_handler;

use std::sync::Arc;

use anyhow::Context;
use application::{
    MovieDiscoveryIndexer, SearchCleanupHandler, SearchReindexHandler, config::AppConfig,
    movies::deps::ReindexSearchDeps, worker::WorkerService,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use domain::ports::{EventHandler, MovieEnrichmentClient, PeriodicJob, PersonEnrichmentClient};

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

    let metadata_client = metadata::create()?;
    let poster_fetcher = poster_fetcher::create()?;
    let object_storage = object_storage::create()?;

    let db = db::connect(&database_url, &backend).await?;
    let (event_publisher_arc, consumer_arc) = event_bus::create(&db.db_pool).await?;

    let image_ref_command = Arc::clone(&db.image_ref_command);
    let image_ref_query = Arc::clone(&db.image_ref_query);

    #[cfg(feature = "federation")]
    let (
        fed_ap_content,
        fed_movie_repo,
        fed_review_repo,
        fed_diary_repo,
        fed_goal_repo,
        fed_stats_repo,
        fed_user_repo,
        base_url,
        allow_registration,
    ) = (
        Arc::clone(&db.ap_content),
        Arc::clone(&db.movie_query),
        Arc::clone(&db.review),
        Arc::clone(&db.diary),
        Arc::clone(&db.goal),
        Arc::clone(&db.stats),
        Arc::clone(&db.user),
        app_config.base_url.clone(),
        app_config.allow_registration,
    );
    // Wire federation repos early to get remote_watchlist_repo for AppContext.
    #[cfg(feature = "federation")]
    let (
        fed_activity_repo,
        fed_follow_repo,
        fed_actor_repo,
        fed_blocklist_repo,
        _fed_social_query,
        fed_review_store,
        fed_remote_watchlist_repo,
    ) = match &db.db_pool {
        #[cfg(feature = "sqlite-federation")]
        db::DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
        #[cfg(feature = "postgres-federation")]
        db::DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
    };

    let movie_command = db.movie_command;
    let movie_query = db.movie_query;
    let deduplicator = db.deduplicator;
    let user = db.user;
    let import_session = db.import_session;
    let movie_profile = db.movie_profile;
    let watch_event_command = db.watch_event_command;
    let watch_event_query = db.watch_event_query;
    let person_command = db.person_command;
    let person_query = db.person_query;
    let search_command = db.search_command;
    let wrapup_stats = db.wrapup_stats;
    let wrapup_repo = db.wrapup_repo;
    let remote_goal = db.remote_goal;
    let refresh_session = db.refresh_session;

    let event_publisher = event_publisher_arc;
    let object_storage = object_storage;
    let metadata = metadata_client;

    // ── Enrichment ────────────────────────────────────────────────────────────
    // Both the event handler and the staleness job are gated on TMDB_API_KEY.
    // Without a key, no MovieEnrichmentRequested events are produced or handled.

    type EnrichmentParts = (
        Option<Arc<dyn EventHandler>>,
        Option<Arc<dyn EventHandler>>,
        Option<Arc<dyn PeriodicJob>>,
    );
    let (enrichment_handler, person_enrichment_handler, enrichment_job): EnrichmentParts =
        match tmdb_enrichment::TmdbEnrichmentClient::from_env() {
            Ok(client) => {
                tracing::info!("TMDb enrichment enabled");
                let client = Arc::new(client);
                let image_fetcher = poster_fetcher::create_image_fetcher()?;
                let handler = Arc::new(application::movies::MovieEnrichmentHandler::new(
                    Arc::clone(&client) as Arc<dyn MovieEnrichmentClient>,
                    Arc::clone(&movie_query),
                    Arc::clone(&movie_profile),
                    Arc::clone(&person_command),
                    Arc::clone(&search_command),
                    Arc::clone(&object_storage),
                    image_fetcher,
                )) as Arc<dyn EventHandler>;
                let person_enrichment_arc = Arc::clone(&client) as Arc<dyn PersonEnrichmentClient>;
                let person_handler = Arc::new(application::person::PersonEnrichmentHandler::new(
                    Arc::clone(&person_query),
                    Some(person_enrichment_arc),
                    Arc::clone(&person_command),
                )) as Arc<dyn EventHandler>;
                let job = Arc::new(application::jobs::EnrichmentStalenessJob::new(
                    Arc::clone(&movie_profile),
                    Arc::clone(&event_publisher),
                )) as Arc<dyn PeriodicJob>;
                (Some(handler), Some(person_handler), Some(job))
            }
            Err(e) => {
                tracing::warn!("TMDb enrichment disabled: {e}");
                (None, None, None)
            }
        };

    // ── Image conversion ──────────────────────────────────────────────────────

    let conversion = image_converter::build(
        Arc::clone(&object_storage),
        image_ref_command,
        image_ref_query,
        Arc::clone(&event_publisher),
    )?;

    // ── Periodic jobs ─────────────────────────────────────────────────────────

    let mut periodic_jobs: Vec<Arc<dyn PeriodicJob>> = vec![
        Arc::new(application::jobs::MovieDeduplicationJob::new(
            Arc::clone(&movie_query),
            Arc::clone(&deduplicator),
            Arc::clone(&object_storage),
        )),
        Arc::new(application::jobs::ImportSessionCleanupJob::new(
            import_session.clone(),
        )),
        Arc::new(application::jobs::WatchEventCleanupJob::new(
            watch_event_command.clone(),
        )),
        Arc::new(application::jobs::WrapUpAutoGenerateJob::new(
            Arc::clone(&user),
            Arc::clone(&wrapup_repo),
            Arc::clone(&event_publisher),
        )),
        Arc::new(application::jobs::WrapUpCleanupJob::new(Arc::clone(
            &wrapup_repo,
        ))),
        Arc::new(application::jobs::RefreshSessionCleanupJob::new(
            Arc::clone(&refresh_session),
        )),
    ];
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
            Arc::clone(&movie_command),
            Arc::clone(&movie_query),
            Arc::clone(&metadata),
            Arc::clone(&poster_fetcher),
            Arc::clone(&object_storage),
            Arc::clone(&event_publisher),
            3,
        )) as Arc<dyn EventHandler>;

        let cleanup = Arc::new(object_storage::ImageCleanupHandler::new(Arc::clone(
            &object_storage,
        ))) as Arc<dyn EventHandler>;

        let search_cleanup = Arc::new(SearchCleanupHandler::new(
            Arc::clone(&search_command),
            Arc::clone(&person_query),
        )) as Arc<dyn EventHandler>;

        let discovery_indexer = Arc::new(MovieDiscoveryIndexer::new(
            Arc::clone(&movie_query),
            Arc::clone(&search_command),
        )) as Arc<dyn EventHandler>;

        let wrapup_handler = Arc::new(application::wrapup::event_handler::WrapUpEventHandler::new(
            Arc::clone(&wrapup_repo),
            Arc::clone(&event_publisher),
            Arc::clone(&wrapup_stats),
        )) as Arc<dyn EventHandler>;

        let reindex_handler = Arc::new(SearchReindexHandler::new(ReindexSearchDeps {
            movie_query: Arc::clone(&movie_query),
            movie_profile: Arc::clone(&movie_profile),
            search_command: Arc::clone(&search_command),
            person_command: Arc::clone(&person_command),
            person_query: Arc::clone(&person_query),
        })) as Arc<dyn EventHandler>;

        let mut h = vec![
            poster,
            cleanup,
            search_cleanup,
            discovery_indexer,
            wrapup_handler,
            reindex_handler,
        ];

        #[cfg(feature = "federation")]
        {
            let ap_wire = activitypub::wire(activitypub::ActivityPubDeps {
                activity_repo: fed_activity_repo,
                follow_repo: fed_follow_repo,
                actor_repo: fed_actor_repo,
                blocklist_repo: fed_blocklist_repo,
                review_store: fed_review_store,
                remote_watchlist_repo: fed_remote_watchlist_repo,
                remote_goal_repo: Arc::clone(&remote_goal),
                local_ap_content: fed_ap_content,
                movie_repo: fed_movie_repo,
                review_repo: fed_review_repo,
                diary_repo: fed_diary_repo,
                goal_repo: fed_goal_repo,
                stats_repo: fed_stats_repo,
                user_repo: fed_user_repo,
                base_url,
                allow_registration,
                event_publisher: Arc::clone(&event_publisher),
                federation_settings: std::sync::Arc::clone(&db.federation_settings),
            })
            .await?;

            tracing::info!("federation event handler registered");
            h.push(ap_wire.event_handler);
            h.push(Arc::new(follow_backfill_handler::FollowBackfillHandler {
                ap_service: ap_wire.service,
            }) as Arc<dyn EventHandler>);
        }

        if let Some(e) = enrichment_handler {
            h.push(e);
        }
        if let Some(e) = person_enrichment_handler {
            h.push(e);
        }
        if let Some((ref conv_handler, _)) = conversion {
            h.push(Arc::clone(conv_handler));
        }

        h
    };

    // ── Run ───────────────────────────────────────────────────────────────────

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = shutdown_tx.send(true);
    });

    let worker = WorkerService::new(consumer_arc, handlers);
    tracing::info!("worker started");
    worker.run(shutdown_rx).await;

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
