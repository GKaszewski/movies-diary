mod event_bus;
mod follow_backfill_handler;

use std::sync::Arc;

use anyhow::Context;
use application::{
    MovieDiscoveryIndexer, SearchCleanupHandler, SearchReindexHandler, config::AppConfig,
    worker::WorkerService,
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
    let instance = domain::value_objects::InstanceIdentity::new(app_config.base_url.clone());

    let metadata_client = metadata::create()?;
    let poster_fetcher = poster_fetcher::create()?;
    let object_storage = object_storage::create()?;

    // TMDb client detection happens before `build_worker_deps` so the optional
    // person-enrichment port can be handed to `WorkerServices` up front.
    let tmdb_client = match tmdb_enrichment::TmdbEnrichmentClient::from_env() {
        Ok(client) => {
            tracing::info!("TMDb enrichment enabled");
            Some(Arc::new(client))
        }
        Err(e) => {
            tracing::warn!("TMDb enrichment disabled: {e}");
            None
        }
    };
    let person_enrichment: Option<Arc<dyn PersonEnrichmentClient>> = tmdb_client
        .clone()
        .map(|c| c as Arc<dyn PersonEnrichmentClient>);

    let db =
        composition::factory::build_database_adapters(&backend, &database_url, &instance).await?;
    let (event_publisher_arc, consumer_arc) = event_bus::create(&db.db_pool).await?;

    let worker_services = application::WorkerServices {
        object_storage: Arc::clone(&object_storage),
        event_publisher: Arc::clone(&event_publisher_arc),
        person_enrichment,
    };
    let worker_deps = composition::build_worker_deps(&db, &worker_services);

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
        allow_registration,
    ) = (
        Arc::clone(&db.ap_content),
        Arc::clone(&db.movie_query),
        Arc::clone(&db.review),
        Arc::clone(&db.diary),
        Arc::clone(&db.goal_query),
        Arc::clone(&db.stats),
        Arc::clone(&db.user),
        app_config.allow_registration,
    );
    // Wire federation repos early to get remote_watchlist_repo for AppContext.
    #[cfg(feature = "federation")]
    let fed_repos = match &db.db_pool {
        #[cfg(feature = "sqlite-federation")]
        composition::DbPool::Sqlite(pool) => {
            sqlite_federation::wire(pool.clone(), instance.clone())
        }
        #[cfg(feature = "postgres-federation")]
        composition::DbPool::Postgres(pool) => {
            postgres_federation::wire(pool.clone(), instance.clone())
        }
    };

    let movie_command = db.movie_command;
    let movie_query = db.movie_query;
    let user = db.user;
    let import_session = db.import_session;
    let movie_profile = db.movie_profile;
    let watch_event_command = db.watch_event_command;
    let person_query = db.person_query;
    let search_command = db.search_command;
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
        match tmdb_client {
            Some(client) => {
                let image_fetcher = poster_fetcher::create_image_fetcher()?;
                let handler = Arc::new(application::movies::MovieEnrichmentHandler::new(
                    Arc::clone(&client) as Arc<dyn MovieEnrichmentClient>,
                    worker_deps.enrich_movie.movie_query.clone(),
                    worker_deps.enrich_movie.movie_profile.clone(),
                    worker_deps.enrich_movie.person_command.clone(),
                    worker_deps.enrich_movie.search_command.clone(),
                    Arc::clone(&object_storage),
                    image_fetcher,
                )) as Arc<dyn EventHandler>;
                let person_handler = Arc::new(application::person::PersonEnrichmentHandler::new(
                    worker_deps.enrich_person.person_query.clone(),
                    worker_deps.enrich_person.person_enrichment.clone(),
                    worker_deps.enrich_person.person_command.clone(),
                )) as Arc<dyn EventHandler>;
                let job = Arc::new(application::jobs::EnrichmentStalenessJob::new(
                    Arc::clone(&movie_profile),
                    Arc::clone(&event_publisher),
                )) as Arc<dyn PeriodicJob>;
                (Some(handler), Some(person_handler), Some(job))
            }
            None => (None, None, None),
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
            worker_deps.merge_duplicates.movie_query.clone(),
            worker_deps.merge_duplicates.deduplicator.clone(),
            worker_deps.merge_duplicates.object_storage.clone(),
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
            worker_deps.handle_requested.wrapup_repo.clone(),
            worker_deps.handle_requested.event_publisher.clone(),
            worker_deps.handle_requested.wrapup_stats.clone(),
        )) as Arc<dyn EventHandler>;

        let reindex_handler = Arc::new(SearchReindexHandler::new(worker_deps.reindex_search))
            as Arc<dyn EventHandler>;

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
                activity_repo: fed_repos.activity,
                follow_repo: fed_repos.follow,
                actor_repo: fed_repos.actor,
                blocklist_repo: fed_repos.blocklist,
                review_store: fed_repos.review_store,
                remote_watchlist_repo: fed_repos.remote_watchlist,
                remote_goal_repo: Arc::clone(&remote_goal),
                local_ap_content: fed_ap_content,
                movie_repo: fed_movie_repo,
                review_repo: fed_review_repo,
                diary_repo: fed_diary_repo,
                goal_repo: fed_goal_repo,
                stats_repo: fed_stats_repo,
                user_repo: fed_user_repo,
                follow_command: fed_repos.follow_command,
                follow_query: fed_repos.follow_query,
                instance: instance.clone(),
                allow_registration,
                event_publisher: Arc::clone(&event_publisher),
                federation_settings: std::sync::Arc::clone(&db.federation_settings),
            })
            .await?;

            tracing::info!("federation event handler registered");
            h.push(ap_wire.event_handler);
            h.push(Arc::new(follow_backfill_handler::FollowBackfillHandler {
                backfill: ap_wire.backfill,
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
    tracing_subscriber::registry()
        .with(log_filter("worker=info,application=info,k_ap=info"))
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
