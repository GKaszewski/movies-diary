use sqlx::SqlitePool;

mod ap_content;
mod diary;
mod goals;
mod image_ref;
mod import_profile;
mod import_session;
mod migrations;
mod models;
mod movie;
mod persons;
mod profile;
mod profile_fields;
mod refresh_sessions;
mod remote_goals;
mod review;
mod stats;
mod user_settings;
mod users;
mod watch_event;
mod watchlist;
mod wrapup;

pub use ap_content::SqliteApContentQuery;
pub use diary::SqliteDiaryRepository;
pub use image_ref::{SqliteImageRefAdapter, create_image_ref};
pub use import_profile::SqliteImportProfileRepository;
pub use import_session::SqliteImportSessionRepository;
pub use movie::SqliteMovieRepository;
pub use persons::{SqlitePersonAdapter, create_person_adapter};
pub use profile::SqliteMovieProfileRepository;
pub use profile_fields::SqliteProfileFieldsRepository;
pub use refresh_sessions::SqliteRefreshSessionAdapter;
pub use review::SqliteReviewRepository;
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

pub(crate) fn format_year_month(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 {
        return ym.to_string();
    }
    let year = parts[0].get(2..).unwrap_or(parts[0]);
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    format!("{} '{}", month, year)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), domain::errors::DomainError> {
    migrations::run(pool).await
}

pub struct SqliteWireOutput {
    pub pool: SqlitePool,
    pub movie: std::sync::Arc<dyn domain::ports::MovieRepository>,
    pub review: std::sync::Arc<dyn domain::ports::ReviewRepository>,
    pub diary: std::sync::Arc<dyn domain::ports::DiaryRepository>,
    pub stats: std::sync::Arc<dyn domain::ports::StatsRepository>,
    pub user: std::sync::Arc<dyn domain::ports::UserRepository>,
    pub import_session: std::sync::Arc<dyn domain::ports::ImportSessionRepository>,
    pub import_profile: std::sync::Arc<dyn domain::ports::ImportProfileRepository>,
    pub movie_profile: std::sync::Arc<dyn domain::ports::MovieProfileRepository>,
    pub watchlist: std::sync::Arc<dyn domain::ports::WatchlistRepository>,
    pub ap_content: std::sync::Arc<dyn domain::ports::LocalApContentQuery>,
    pub wrapup_repo: std::sync::Arc<dyn domain::ports::WrapUpRepository>,
    pub wrapup_stats: std::sync::Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub goal: std::sync::Arc<dyn domain::ports::GoalRepository>,
    pub user_settings: std::sync::Arc<dyn domain::ports::UserSettingsRepository>,
    pub remote_goal: std::sync::Arc<dyn domain::ports::RemoteGoalRepository>,
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

    Ok(SqliteWireOutput {
        pool: pool.clone(),
        movie: std::sync::Arc::new(SqliteMovieRepository::new(pool.clone())) as _,
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
        goal: std::sync::Arc::new(goals::SqliteGoalRepository::new(pool.clone())) as _,
        user_settings: std::sync::Arc::new(user_settings::SqliteUserSettingsRepository::new(
            pool.clone(),
        )) as _,
        remote_goal: std::sync::Arc::new(remote_goals::SqliteRemoteGoalRepository::new(pool)) as _,
    })
}
