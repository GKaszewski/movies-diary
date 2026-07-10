use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    DiaryRepository, GoalRepository, ImageRefCommand, ImageRefQuery, ImportSessionRepository,
    LocalApContentQuery, MovieCommand, MovieDeduplicator, MovieProfileRepository, MovieQuery,
    PersonCommand, PersonQuery, ReviewRepository, SearchCommand, StatsRepository, UserRepository,
    WatchEventCommand, WatchEventQuery,
};

pub use infra_wiring::DbPool;

pub struct WorkerDbOutput {
    pub movie_command: Arc<dyn MovieCommand>,
    pub movie_query: Arc<dyn MovieQuery>,
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub stats: Arc<dyn StatsRepository>,
    pub goal: Arc<dyn GoalRepository>,
    pub user: Arc<dyn UserRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_command: Arc<dyn SearchCommand>,
    pub ap_content: Arc<dyn LocalApContentQuery>,
    pub image_ref_command: Arc<dyn ImageRefCommand>,
    pub image_ref_query: Arc<dyn ImageRefQuery>,
    pub wrapup_stats: Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn domain::ports::WrapUpRepository>,
    pub remote_goal: Arc<dyn domain::ports::RemoteGoalRepository>,
    pub refresh_session: Arc<dyn domain::ports::RefreshSessionRepository>,
    pub federation_settings: Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub deduplicator: Arc<dyn MovieDeduplicator>,
    pub db_pool: DbPool,
}

pub async fn connect(database_url: &str, backend: &str) -> anyhow::Result<WorkerDbOutput> {
    match backend {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let w = postgres::wire(database_url)
                .await
                .context("PostgreSQL connection failed")?;
            let (image_ref_command, image_ref_query) = postgres::create_image_ref(w.pool.clone());
            let (person_command, person_query) = postgres::create_person_adapter(w.pool.clone());
            let (search_command, _search_port) =
                postgres_search::create_search_adapter(w.pool.clone());
            let we = Arc::new(postgres::PostgresWatchEventRepository::new(w.pool.clone()));
            Ok(WorkerDbOutput {
                movie_command: w.movie_command,
                movie_query: w.movie_query,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                goal: w.goal,
                user: w.user,
                import_session: w.import_session,
                movie_profile: w.movie_profile,
                watch_event_command: we.clone() as _,
                watch_event_query: we as _,
                person_command,
                person_query,
                search_command,
                ap_content: w.ap_content,
                image_ref_command,
                image_ref_query,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                remote_goal: w.remote_goal,
                refresh_session: Arc::new(postgres::PostgresRefreshSessionAdapter::new(
                    w.pool.clone(),
                )) as _,
                federation_settings: w.federation_settings,
                deduplicator: w.deduplicator,
                db_pool: DbPool::Postgres(w.pool),
            })
        }
        #[cfg(feature = "sqlite")]
        _ => {
            let w = sqlite::wire(database_url)
                .await
                .context("SQLite connection failed")?;
            let (image_ref_command, image_ref_query) = sqlite::create_image_ref(w.pool.clone());
            let (person_command, person_query) = sqlite::create_person_adapter(w.pool.clone());
            let (search_command, _search_port) =
                sqlite_search::create_search_adapter(w.pool.clone());
            let we = Arc::new(sqlite::SqliteWatchEventRepository::new(w.pool.clone()));
            Ok(WorkerDbOutput {
                movie_command: w.movie_command,
                movie_query: w.movie_query,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                goal: w.goal,
                user: w.user,
                import_session: w.import_session,
                movie_profile: w.movie_profile,
                watch_event_command: we.clone() as _,
                watch_event_query: we as _,
                person_command,
                person_query,
                search_command,
                ap_content: w.ap_content,
                image_ref_command,
                image_ref_query,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                remote_goal: w.remote_goal,
                refresh_session: Arc::new(sqlite::SqliteRefreshSessionAdapter::new(w.pool.clone()))
                    as _,
                federation_settings: w.federation_settings,
                deduplicator: w.deduplicator,
                db_pool: DbPool::Sqlite(w.pool),
            })
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build"),
    }
}
