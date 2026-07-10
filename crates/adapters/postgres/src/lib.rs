use domain::errors::DomainError;
use sqlx::PgPool;

mod diary;
mod goals;
mod image_ref;
mod import_profile;
mod import_session;
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

pub use diary::PostgresDiaryRepository;
pub use image_ref::{PostgresImageRefAdapter, create_image_ref};
pub use import_profile::PostgresImportProfileRepository;
pub use import_session::PostgresImportSessionRepository;
pub use movie::PostgresMovieRepository;
pub use movie_dedup::PostgresMovieDeduplicator;
pub use persons::{PostgresPersonAdapter, create_person_adapter};
pub use postgres_federation::PostgresApContentQuery;
pub use profile::PostgresMovieProfileRepository;
pub use profile_fields::PostgresProfileFieldsRepository;
pub use refresh_sessions::PostgresRefreshSessionAdapter;
pub use review::PostgresReviewRepository;
pub use stats::PostgresStatsRepository;
pub use users::PostgresUserRepository;
pub use watch_event::{PostgresWatchEventRepository, PostgresWebhookTokenRepository};
pub use watchlist::PostgresWatchlistRepository;
pub use wrapup::{PostgresWrapUpRepository, PostgresWrapUpStatsQuery};

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

pub async fn wire(database_url: &str) -> anyhow::Result<PostgresWireOutput> {
    use anyhow::Context;

    let pool = sqlx::PgPool::connect(database_url)
        .await
        .context("Failed to connect to PostgreSQL database")?;

    migrate(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("Database migration failed")?;

    let user_settings_repo = std::sync::Arc::new(
        user_settings::PostgresUserSettingsRepository::new(pool.clone()),
    );

    let movie_repo = std::sync::Arc::new(PostgresMovieRepository::new(pool.clone()));

    Ok(PostgresWireOutput {
        pool: pool.clone(),
        movie_command: movie_repo.clone() as _,
        movie_query: movie_repo as _,
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
        goal_command: std::sync::Arc::new(goals::PostgresGoalRepository::new(pool.clone())) as _,
        goal_query: std::sync::Arc::new(goals::PostgresGoalRepository::new(pool.clone())) as _,
        user_settings: std::sync::Arc::clone(&user_settings_repo) as _,
        federation_settings: user_settings_repo as _,
        remote_goal: std::sync::Arc::new(postgres_federation::PostgresRemoteGoalRepository::new(
            pool.clone(),
        )) as _,
        deduplicator: std::sync::Arc::new(PostgresMovieDeduplicator::new(pool)) as _,
    })
}
