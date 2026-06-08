use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    DiaryRepository, ImageRefCommand, ImageRefQuery, ImportProfileRepository,
    ImportSessionRepository, LocalApContentQuery, MovieProfileRepository, MovieRepository,
    PersonCommand, PersonQuery, ReviewRepository, SearchCommand, SearchPort, StatsRepository,
    UserProfileFieldsRepository, UserRepository, WatchEventRepository, WatchlistRepository,
    WebhookTokenRepository,
};

pub enum DbPool {
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
}

pub struct WorkerDbOutput {
    pub movie: Arc<dyn MovieRepository>,
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub stats: Arc<dyn StatsRepository>,
    pub user: Arc<dyn UserRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub import_profile: Arc<dyn ImportProfileRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub watch_event: Arc<dyn WatchEventRepository>,
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_command: Arc<dyn SearchCommand>,
    pub search_port: Arc<dyn SearchPort>,
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub ap_content: Arc<dyn LocalApContentQuery>,
    pub image_ref_command: Arc<dyn ImageRefCommand>,
    pub image_ref_query: Arc<dyn ImageRefQuery>,
    pub wrapup_stats: Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn domain::ports::WrapUpRepository>,
    pub goal: Arc<dyn domain::ports::GoalRepository>,
    pub user_settings: Arc<dyn domain::ports::UserSettingsRepository>,
    pub remote_goal: Arc<dyn domain::ports::RemoteGoalRepository>,
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
            let (search_command, search_port) =
                postgres_search::create_search_adapter(w.pool.clone());
            let pf = postgres::create_profile_fields_repo(w.pool.clone());
            let we: Arc<dyn WatchEventRepository> =
                Arc::new(postgres::PostgresWatchEventRepository::new(w.pool.clone()));
            let wt: Arc<dyn WebhookTokenRepository> = Arc::new(
                postgres::PostgresWebhookTokenRepository::new(w.pool.clone()),
            );
            Ok(WorkerDbOutput {
                movie: w.movie,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                user: w.user,
                import_session: w.import_session,
                import_profile: w.import_profile,
                movie_profile: w.movie_profile,
                watchlist: w.watchlist,
                watch_event: we,
                webhook_token: wt,
                person_command,
                person_query,
                search_command,
                search_port,
                profile_fields: pf,
                ap_content: w.ap_content,
                image_ref_command,
                image_ref_query,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                goal: w.goal,
                user_settings: w.user_settings,
                remote_goal: w.remote_goal,
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
            let (search_command, search_port) =
                sqlite_search::create_search_adapter(w.pool.clone());
            let pf = sqlite::create_profile_fields_repo(w.pool.clone());
            let we: Arc<dyn WatchEventRepository> =
                Arc::new(sqlite::SqliteWatchEventRepository::new(w.pool.clone()));
            let wt: Arc<dyn WebhookTokenRepository> =
                Arc::new(sqlite::SqliteWebhookTokenRepository::new(w.pool.clone()));
            Ok(WorkerDbOutput {
                movie: w.movie,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                user: w.user,
                import_session: w.import_session,
                import_profile: w.import_profile,
                movie_profile: w.movie_profile,
                watchlist: w.watchlist,
                watch_event: we,
                webhook_token: wt,
                person_command,
                person_query,
                search_command,
                search_port,
                profile_fields: pf,
                ap_content: w.ap_content,
                image_ref_command,
                image_ref_query,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                goal: w.goal,
                user_settings: w.user_settings,
                remote_goal: w.remote_goal,
                db_pool: DbPool::Sqlite(w.pool),
            })
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("DATABASE_BACKEND={backend} is not supported by this build"),
    }
}
