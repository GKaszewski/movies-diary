use sqlx::SqlitePool;

mod diary;
mod goals;
mod image_ref;
mod import_profile;
mod import_session;
mod migrations;
mod models;
mod movie;
mod movie_dedup;
mod persons;
mod profile;
mod profile_fields;
mod refresh_sessions;
mod review;
mod stats;
mod user_settings;
mod users;
mod watch_event;
mod watchlist;
mod wrapup;

pub use diary::SqliteDiaryRepository;
pub use image_ref::{SqliteImageRefAdapter, create_image_ref};
pub use import_profile::SqliteImportProfileRepository;
pub use import_session::SqliteImportSessionRepository;
pub use movie::SqliteMovieRepository;
pub use movie_dedup::SqliteMovieDeduplicator;
pub use persons::{SqlitePersonAdapter, create_person_adapter};
pub use profile::SqliteMovieProfileRepository;
pub use profile_fields::SqliteProfileFieldsRepository;
pub use refresh_sessions::SqliteRefreshSessionAdapter;
pub use review::SqliteReviewRepository;
pub use sqlite_federation::SqliteApContentQuery;
pub use stats::SqliteStatsRepository;
pub use users::SqliteUserRepository;
pub use watch_event::{SqliteWatchEventRepository, SqliteWebhookTokenRepository};
pub use watchlist::SqliteWatchlistRepository;
pub use wrapup::{SqliteWrapUpRepository, SqliteWrapUpStatsQuery};

pub fn create_profile_fields_repo(
    pool: sqlx::SqlitePool,
) -> std::sync::Arc<dyn domain::ports::UserProfileFieldsRepository> {
    std::sync::Arc::new(SqliteProfileFieldsRepository::new(pool))
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), domain::errors::DomainError> {
    migrations::run(pool).await
}

pub struct SqliteWireOutput {
    pub pool: SqlitePool,
    pub movie_command: std::sync::Arc<dyn domain::ports::MovieCommand>,
    pub movie_query: std::sync::Arc<dyn domain::ports::MovieQuery>,
    pub review: std::sync::Arc<dyn domain::ports::ReviewRepository>,
    pub diary: std::sync::Arc<dyn domain::ports::DiaryQuery>,
    pub stats: std::sync::Arc<dyn domain::ports::StatsRepository>,
    pub user: std::sync::Arc<dyn domain::ports::UserRepository>,
    pub import_session: std::sync::Arc<dyn domain::ports::ImportSessionRepository>,
    pub import_profile: std::sync::Arc<dyn domain::ports::ImportProfileRepository>,
    pub movie_profile: std::sync::Arc<dyn domain::ports::MovieProfileRepository>,
    pub watchlist: std::sync::Arc<dyn domain::ports::WatchlistRepository>,
    pub ap_content: std::sync::Arc<dyn domain::ports::LocalApContentQuery>,
    pub wrapup_repo: std::sync::Arc<dyn domain::ports::WrapUpRepository>,
    pub wrapup_stats: std::sync::Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub goal_command: std::sync::Arc<dyn domain::ports::GoalCommand>,
    pub goal_query: std::sync::Arc<dyn domain::ports::GoalQuery>,
    pub user_settings: std::sync::Arc<dyn domain::ports::UserSettingsRepository>,
    pub federation_settings: std::sync::Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub remote_goal: std::sync::Arc<dyn domain::ports::RemoteGoalRepository>,
    pub deduplicator: std::sync::Arc<dyn domain::ports::MovieDeduplicator>,
}

pub async fn wire(database_url: &str) -> anyhow::Result<SqliteWireOutput> {
    use anyhow::Context;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let opts = SqliteConnectOptions::from_str(database_url)
        .context("Invalid DATABASE_URL")?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::pool::PoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .context("Failed to connect to SQLite database")?;

    migrate(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Database migration failed")?;

    let user_settings_repo = std::sync::Arc::new(user_settings::SqliteUserSettingsRepository::new(
        pool.clone(),
    ));

    let movie_repo = std::sync::Arc::new(SqliteMovieRepository::new(pool.clone()));

    Ok(SqliteWireOutput {
        pool: pool.clone(),
        movie_command: movie_repo.clone() as _,
        movie_query: movie_repo as _,
        review: std::sync::Arc::new(SqliteReviewRepository::new(pool.clone())) as _,
        diary: std::sync::Arc::new(SqliteDiaryRepository::new(pool.clone())) as _,
        stats: std::sync::Arc::new(SqliteStatsRepository::new(pool.clone())) as _,
        user: std::sync::Arc::new(SqliteUserRepository::new(pool.clone())) as _,
        import_session: std::sync::Arc::new(SqliteImportSessionRepository::new(pool.clone())) as _,
        import_profile: std::sync::Arc::new(SqliteImportProfileRepository::new(pool.clone())) as _,
        movie_profile: std::sync::Arc::new(SqliteMovieProfileRepository::new(pool.clone())) as _,
        watchlist: std::sync::Arc::new(SqliteWatchlistRepository::new(pool.clone())) as _,
        ap_content: std::sync::Arc::new(SqliteApContentQuery::new(pool.clone())) as _,
        wrapup_repo: std::sync::Arc::new(SqliteWrapUpRepository::new(pool.clone())) as _,
        wrapup_stats: std::sync::Arc::new(SqliteWrapUpStatsQuery::new(pool.clone())) as _,
        goal_command: std::sync::Arc::new(goals::SqliteGoalRepository::new(pool.clone())) as _,
        goal_query: std::sync::Arc::new(goals::SqliteGoalRepository::new(pool.clone())) as _,
        user_settings: std::sync::Arc::clone(&user_settings_repo) as _,
        federation_settings: user_settings_repo as _,
        remote_goal: std::sync::Arc::new(sqlite_federation::SqliteRemoteGoalRepository::new(
            pool.clone(),
        )) as _,
        deduplicator: std::sync::Arc::new(SqliteMovieDeduplicator::new(pool)) as _,
    })
}
