use domain::errors::DomainError;
use sqlx::PgPool;

mod ap_content;
mod diary;
mod goals;
mod image_ref;
mod import_profile;
mod import_session;
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

pub use ap_content::PostgresApContentQuery;
pub use diary::PostgresDiaryRepository;
pub use image_ref::{PostgresImageRefAdapter, create_image_ref};
pub use import_profile::PostgresImportProfileRepository;
pub use import_session::PostgresImportSessionRepository;
pub use movie::PostgresMovieRepository;
pub use persons::{PostgresPersonAdapter, create_person_adapter};
pub use profile::PostgresMovieProfileRepository;
pub use profile_fields::PostgresProfileFieldsRepository;
pub use refresh_sessions::PostgresRefreshSessionAdapter;
pub use review::PostgresReviewRepository;
pub use stats::PostgresStatsRepository;
pub use users::PostgresUserRepository;
pub use watch_event::{PostgresWatchEventRepository, PostgresWebhookTokenRepository};
pub use watchlist::PostgresWatchlistRepository;
pub use wrapup::{PostgresWrapUpRepository, PostgresWrapUpStatsQuery};

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

pub async fn migrate(pool: &PgPool) -> Result<(), DomainError> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))
}

pub fn create_profile_fields_repo(
    pool: sqlx::PgPool,
) -> std::sync::Arc<dyn domain::ports::UserProfileFieldsRepository> {
    std::sync::Arc::new(profile_fields::PostgresProfileFieldsRepository::new(pool))
}

pub struct PostgresWireOutput {
    pub pool: PgPool,
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
    pub federation_settings: std::sync::Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub remote_goal: std::sync::Arc<dyn domain::ports::RemoteGoalRepository>,
}

pub async fn wire(database_url: &str) -> anyhow::Result<PostgresWireOutput> {
    use anyhow::Context;

    let pool = sqlx::PgPool::connect(database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    migrate(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Database migration failed")?;

    let user_settings_repo = std::sync::Arc::new(user_settings::PostgresUserSettingsRepository::new(pool.clone()));

    Ok(PostgresWireOutput {
        pool: pool.clone(),
        movie: std::sync::Arc::new(PostgresMovieRepository::new(pool.clone())) as _,
        review: std::sync::Arc::new(PostgresReviewRepository::new(pool.clone())) as _,
        diary: std::sync::Arc::new(PostgresDiaryRepository::new(pool.clone())) as _,
        stats: std::sync::Arc::new(PostgresStatsRepository::new(pool.clone())) as _,
        user: std::sync::Arc::new(PostgresUserRepository::new(pool.clone())) as _,
        import_session: std::sync::Arc::new(PostgresImportSessionRepository::new(pool.clone()))
            as _,
        import_profile: std::sync::Arc::new(PostgresImportProfileRepository::new(pool.clone()))
            as _,
        movie_profile: std::sync::Arc::new(PostgresMovieProfileRepository::new(pool.clone())) as _,
        watchlist: std::sync::Arc::new(PostgresWatchlistRepository::new(pool.clone())) as _,
        ap_content: std::sync::Arc::new(PostgresApContentQuery::new(pool.clone())) as _,
        wrapup_repo: std::sync::Arc::new(PostgresWrapUpRepository::new(pool.clone())) as _,
        wrapup_stats: std::sync::Arc::new(PostgresWrapUpStatsQuery::new(pool.clone())) as _,
        goal: std::sync::Arc::new(goals::PostgresGoalRepository::new(pool.clone())) as _,
        user_settings: std::sync::Arc::clone(&user_settings_repo) as _,
        federation_settings: user_settings_repo as _,
        remote_goal: std::sync::Arc::new(remote_goals::PostgresRemoteGoalRepository::new(pool))
            as _,
    })
}
