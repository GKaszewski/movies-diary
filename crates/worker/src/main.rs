mod db;
mod event_bus;
mod follow_backfill_handler;

use std::sync::Arc;

use anyhow::Context;
use application::{
    MovieDiscoveryIndexer, SearchCleanupHandler,
    config::AppConfig,
    context::{AppContext, Repositories, Services},
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

    let db = db::connect(&database_url, &backend).await?;
    let (event_publisher_arc, consumer_arc) = event_bus::create(&db.db_pool).await?;

    let image_ref_command = Arc::clone(&db.image_ref_command);
    let image_ref_query = Arc::clone(&db.image_ref_query);

    #[cfg(feature = "federation")]
    let (fed_ap_content, fed_user_repo, base_url, allow_registration) = (
        Arc::clone(&db.ap_content),
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
        fed_social_query,
        fed_review_store,
        fed_remote_watchlist_repo,
    ) = match &db.db_pool {
        #[cfg(feature = "sqlite-federation")]
        db::DbPool::Sqlite(pool) => sqlite_federation::wire(pool.clone()),
        #[cfg(feature = "postgres-federation")]
        db::DbPool::Postgres(pool) => postgres_federation::wire(pool.clone()),
    };

    let ctx = AppContext {
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
            profile_fields: db.profile_fields,
            person_command: db.person_command,
            person_query: db.person_query,
            search_port: db.search_port,
            search_command: db.search_command,
            #[cfg(feature = "federation")]
            remote_watchlist: fed_remote_watchlist_repo.clone(),
            #[cfg(not(feature = "federation"))]
            remote_watchlist: Arc::new(domain::testing::NoopRemoteWatchlistRepository),
            #[cfg(feature = "federation")]
            social_query: fed_social_query,
            #[cfg(not(feature = "federation"))]
            social_query: Arc::new(domain::testing::NoopSocialQueryPort),
            wrapup_stats: db.wrapup_stats,
            wrapup_repo: db.wrapup_repo,
        },
        services: Services {
            auth: auth_service,
            password_hasher,
            metadata: metadata_client,
            poster_fetcher,
            image_storage,
            event_publisher: event_publisher_arc,
            diary_exporter: Arc::new(ExportAdapter) as Arc<dyn DiaryExporter>,
            document_parser: Arc::new(ImporterDocumentParser) as Arc<dyn DocumentParser>,
            video_renderer: {
                let wc = &app_config.wrapup;
                let ffmpeg = &wc.ffmpeg_path;
                if std::process::Command::new(ffmpeg)
                    .arg("-version")
                    .output()
                    .is_ok()
                {
                    let renderer_cfg = wrapup_renderer::RendererConfig {
                        slide_duration_secs: 4,
                        transition_duration_secs: 0.8,
                        resolution: (1080, 1920),
                        ffmpeg_path: ffmpeg.clone(),
                        font_path: wc.font_path.clone(),
                        logo_path: wc.logo_path.clone(),
                        bg_dir: wc.bg_dir.clone(),
                    };
                    match wrapup_renderer::FfmpegWrapUpRenderer::new(renderer_cfg) {
                        Ok(r) => {
                            tracing::info!("wrapup video renderer enabled (ffmpeg={ffmpeg})");
                            Some(Arc::new(r) as Arc<dyn domain::ports::WrapUpVideoRenderer>)
                        }
                        Err(e) => {
                            tracing::warn!("wrapup video renderer init failed: {e}");
                            None
                        }
                    }
                } else {
                    tracing::info!("wrapup video renderer disabled (ffmpeg not found)");
                    None
                }
            },
        },
        config: app_config,
    };

    // ── Enrichment ────────────────────────────────────────────────────────────
    // Both the event handler and the staleness job are gated on TMDB_API_KEY.
    // Without a key, no MovieEnrichmentRequested events are produced or handled.

    type OptionalPair = (Option<Arc<dyn EventHandler>>, Option<Arc<dyn PeriodicJob>>);
    let (enrichment_handler, enrichment_job): OptionalPair =
        match tmdb_enrichment::TmdbEnrichmentClient::from_env() {
            Ok(client) => {
                tracing::info!("TMDb enrichment enabled");
                let handler = Arc::new(tmdb_enrichment::EnrichmentHandler::new(
                    Arc::new(client),
                    Arc::clone(&ctx.repos.movie),
                    Arc::clone(&ctx.repos.movie_profile),
                    Arc::clone(&ctx.repos.person_command),
                    Arc::clone(&ctx.repos.search_command),
                    Arc::clone(&ctx.services.image_storage),
                )) as Arc<dyn EventHandler>;
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
        Arc::clone(&ctx.services.image_storage),
        image_ref_command,
        image_ref_query,
        Arc::clone(&ctx.services.event_publisher),
    )?;

    // ── Periodic jobs ─────────────────────────────────────────────────────────

    let mut periodic_jobs: Vec<Arc<dyn PeriodicJob>> = vec![
        Arc::new(application::jobs::ImportSessionCleanupJob::new(ctx.clone())),
        Arc::new(application::jobs::WatchEventCleanupJob::new(ctx.clone())),
        Arc::new(application::jobs::WrapUpAutoGenerateJob::new(ctx.clone())),
        Arc::new(application::jobs::WrapUpCleanupJob::new(ctx.clone())),
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
            Arc::clone(&ctx.repos.movie),
            Arc::clone(&ctx.services.metadata),
            Arc::clone(&ctx.services.poster_fetcher),
            Arc::clone(&ctx.services.image_storage),
            Arc::clone(&ctx.services.event_publisher),
            3,
        )) as Arc<dyn EventHandler>;

        let cleanup = Arc::new(image_storage::ImageCleanupHandler::new(Arc::clone(
            &ctx.services.image_storage,
        ))) as Arc<dyn EventHandler>;

        #[cfg(not(feature = "federation"))]
        {
            let search_cleanup = Arc::new(SearchCleanupHandler::new(
                Arc::clone(&ctx.repos.search_command),
                Arc::clone(&ctx.repos.person_query),
            )) as Arc<dyn EventHandler>;
            let discovery_indexer = Arc::new(MovieDiscoveryIndexer::new(
                Arc::clone(&ctx.repos.movie),
                Arc::clone(&ctx.repos.search_command),
            )) as Arc<dyn EventHandler>;
            let wrapup_handler = Arc::new(
                application::wrapup::event_handler::WrapUpEventHandler::new(ctx.clone()),
            ) as Arc<dyn EventHandler>;
            let mut h = vec![
                poster,
                cleanup,
                search_cleanup,
                discovery_indexer,
                wrapup_handler,
            ];
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
            let ap_wire = activitypub::wire(activitypub::ActivityPubDeps {
                activity_repo: fed_activity_repo,
                follow_repo: fed_follow_repo,
                actor_repo: fed_actor_repo,
                blocklist_repo: fed_blocklist_repo,
                review_store: fed_review_store,
                remote_watchlist_repo: fed_remote_watchlist_repo,
                local_ap_content: fed_ap_content,
                user_repo: fed_user_repo,
                base_url,
                allow_registration,
                event_publisher: Arc::clone(&ctx.services.event_publisher),
            })
            .await?;

            let ap_event_handler = ap_wire.event_handler;
            let backfill = Arc::new(follow_backfill_handler::FollowBackfillHandler {
                ap_service: ap_wire.service,
            }) as Arc<dyn EventHandler>;

            let search_cleanup = Arc::new(SearchCleanupHandler::new(
                Arc::clone(&ctx.repos.search_command),
                Arc::clone(&ctx.repos.person_query),
            )) as Arc<dyn EventHandler>;
            let discovery_indexer = Arc::new(MovieDiscoveryIndexer::new(
                Arc::clone(&ctx.repos.movie),
                Arc::clone(&ctx.repos.search_command),
            )) as Arc<dyn EventHandler>;
            tracing::info!("federation event handler registered");
            let wrapup_handler = Arc::new(
                application::wrapup::event_handler::WrapUpEventHandler::new(ctx.clone()),
            ) as Arc<dyn EventHandler>;
            let mut h = vec![
                poster,
                cleanup,
                ap_event_handler,
                backfill,
                search_cleanup,
                discovery_indexer,
                wrapup_handler,
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
