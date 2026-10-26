use std::sync::Arc;

use application::{
    MovieDiscoveryIndexer, SearchCleanupHandler, SearchReindexHandler, worker::WorkerService,
};
use domain::ports::{EventHandler, MovieEnrichmentClient, PeriodicJob, PersonEnrichmentClient};
use domain::value_objects::InstanceIdentity;

use crate::database::{self, DatabaseOutput};
use crate::event_bus;
use crate::outbox_relay::OutboxRelay;
use tokio::sync::watch;

pub struct WorkerApp {
    service: WorkerService,
    relay: Option<OutboxRelay>,
}

impl WorkerApp {
    pub fn handler_names(&self) -> Vec<&'static str> {
        self.service.handler_names()
    }

    pub async fn run(self, mut shutdown: watch::Receiver<bool>) -> anyhow::Result<()> {
        let (stop_tx, stop_rx) = watch::channel(false);
        let mut relay = self.relay.map(|r| tokio::spawn(r.run(stop_rx.clone())));
        let service = self.service.run(stop_rx);
        tokio::pin!(service);

        let mut stopping = false;
        let mut relay_failure = None;
        loop {
            tokio::select! {
                _ = &mut service => break,
                _ = shutdown.changed(), if !stopping => {
                    stopping = true;
                    let _ = stop_tx.send(true);
                }
                result = async { relay.as_mut().expect("guarded").await }, if relay.is_some() => {
                    relay = None;
                    if !stopping {
                        relay_failure = Some(match result {
                            Ok(()) => "outbox relay stopped unexpectedly".to_owned(),
                            Err(e) => format!("outbox relay died: {e}"),
                        });
                        stopping = true;
                        let _ = stop_tx.send(true);
                    }
                }
            }
        }
        let _ = stop_tx.send(true);
        if let Some(handle) = relay
            && let Err(e) = handle.await
        {
            relay_failure.get_or_insert(format!("outbox relay died: {e}"));
        }
        match relay_failure {
            Some(message) => {
                tracing::error!("{message}");
                Err(anyhow::anyhow!(message))
            }
            None => Ok(()),
        }
    }
}

pub async fn build_worker(config: &config::AppConfig) -> anyhow::Result<WorkerApp> {
    database::warn_if_database_backend_set();
    let instance = InstanceIdentity::new(config.server.base_url.clone());
    let db = database::build_database_adapters(&config.database.url, &instance).await?;
    assemble(config, db, instance).await
}

pub async fn build_worker_on(
    config: &config::AppConfig,
    pool: sqlx::SqlitePool,
) -> anyhow::Result<WorkerApp> {
    let instance = InstanceIdentity::new(config.server.base_url.clone());
    let db = database::build_database_adapters_on(pool, &instance).await?;
    assemble(config, db, instance).await
}

async fn assemble(
    config: &config::AppConfig,
    db: DatabaseOutput,
    #[cfg_attr(not(feature = "federation"), allow(unused_variables))] instance: InstanceIdentity,
) -> anyhow::Result<WorkerApp> {
    crate::tls::install_crypto_provider();
    let metadata_client = metadata::create(&config.metadata)?;
    let poster_fetcher = poster_fetcher::create(&config.metadata)?;
    let object_storage = object_storage::create(&config.storage)?;

    // TMDb client detection happens before `build_worker_deps` so the optional
    // person-enrichment port can be handed to `WorkerServices` up front.
    let tmdb_client = match tmdb_enrichment::TmdbEnrichmentClient::from_config(&config.metadata) {
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

    let (event_publisher_arc, consumer_arc, relay) =
        event_bus::create_channel(&db.db_pool, config).await?;

    let worker_services = application::WorkerServices {
        object_storage: Arc::clone(&object_storage),
        event_publisher: Arc::clone(&event_publisher_arc),
        person_enrichment,
    };
    let worker_deps = crate::build::build_worker_deps(&db, &worker_services);

    let image_ref_command = Arc::clone(&db.image_ref_command);
    let image_ref_query = Arc::clone(&db.image_ref_query);

    #[cfg(feature = "federation")]
    let (federation_handlers, deletion_notifier) =
        crate::federation::wire_worker_handlers(&db, &instance, config, &event_publisher_arc)
            .await?;
    #[cfg(not(feature = "federation"))]
    let deletion_notifier: Arc<dyn domain::ports::FederationDeletionNotifier> =
        Arc::new(domain::ports::noop::NoopFederationDeletionNotifier);

    let movie_command = db.movie_command;
    let movie_query = db.movie_query;
    let user = db.user;
    let import_session = db.import_session;
    let movie_profile = db.movie_profile;
    let watch_event_command = db.watch_event_command;
    let person_query = db.person_query;
    let search_command = db.search_command;
    let wrapup_repo = db.wrapup_repo;
    let refresh_session = db.refresh_session;
    let processed_events = db.processed_events;
    let account_purger = db.account_purger;

    let event_publisher = event_publisher_arc;
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
                let image_fetcher = poster_fetcher::create_image_fetcher(&config.metadata)?;
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
        &config.image_conversion,
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
        )),
        Arc::new(application::jobs::WrapUpCleanupJob::new(Arc::clone(
            &wrapup_repo,
        ))),
        Arc::new(application::jobs::RefreshSessionCleanupJob::new(
            Arc::clone(&refresh_session),
        )),
        Arc::new(application::jobs::ProcessedEventsCleanupJob::new(
            Arc::clone(&processed_events),
        )),
        Arc::new(application::jobs::TombstoneCleanupJob::new(Arc::clone(
            &account_purger,
        ))),
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
        let poster = Arc::new(
            application::movies::poster_sync_handler::PosterSyncHandler::new(
                Arc::clone(&movie_command),
                Arc::clone(&movie_query),
                Arc::clone(&metadata),
                Arc::clone(&poster_fetcher),
                Arc::clone(&object_storage),
                Arc::clone(&event_publisher),
                3,
            ),
        ) as Arc<dyn EventHandler>;

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
            worker_deps.handle_requested.wrapup_stats.clone(),
        )) as Arc<dyn EventHandler>;

        let reindex_handler = Arc::new(SearchReindexHandler::new(worker_deps.reindex_search))
            as Arc<dyn EventHandler>;

        let account_purge = Arc::new(application::users::purge_account::AccountPurgeHandler::new(
            Arc::clone(&user),
            deletion_notifier,
            Arc::clone(&account_purger),
            Arc::clone(&object_storage),
        )) as Arc<dyn EventHandler>;

        let mut h = vec![
            account_purge,
            poster,
            cleanup,
            search_cleanup,
            discovery_indexer,
            wrapup_handler,
            reindex_handler,
        ];

        #[cfg(feature = "federation")]
        h.extend(federation_handlers);

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

    Ok(WorkerApp {
        service: WorkerService::new(consumer_arc, handlers, processed_events)?,
        relay,
    })
}
