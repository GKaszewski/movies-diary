use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    ImageRefCommand, ImageRefQuery, ImportSessionRepository, LocalApContentQuery,
    MovieDeduplicator, MovieProfileRepository, MovieRepository, PersonCommand, PersonQuery,
    SearchCommand, UserRepository, WatchEventRepository,
};

pub enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

pub struct WorkerDbOutput {
    pub movie: Arc<dyn MovieRepository>,
    pub user: Arc<dyn UserRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub watch_event: Arc<dyn WatchEventRepository>,
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
            let we: Arc<dyn WatchEventRepository> =
                Arc::new(postgres::PostgresWatchEventRepository::new(w.pool.clone()));
            Ok(WorkerDbOutput {
                movie: w.movie,
                user: w.user,
                import_session: w.import_session,
                movie_profile: w.movie_profile,
                watch_event: we,
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
            let we: Arc<dyn WatchEventRepository> =
                Arc::new(sqlite::SqliteWatchEventRepository::new(w.pool.clone()));
            Ok(WorkerDbOutput {
                movie: w.movie,
                user: w.user,
                import_session: w.import_session,
                movie_profile: w.movie_profile,
                watch_event: we,
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
