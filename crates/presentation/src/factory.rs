use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    AuthService, LocalApContentQuery, MetadataClient, ObjectStorage, PasswordHasher,
    PosterFetcherClient, RefreshSessionRepository, UserProfileFieldsRepository,
    WatchEventCommand, WatchEventQuery, WebhookTokenRepository,
};

pub use infra_wiring::DbPool;

pub struct DatabaseOutput {
    pub movie_command: Arc<dyn domain::ports::MovieCommand>,
    pub movie_query: Arc<dyn domain::ports::MovieQuery>,
    pub review: Arc<dyn domain::ports::ReviewRepository>,
    pub diary: Arc<dyn domain::ports::DiaryRepository>,
    pub stats: Arc<dyn domain::ports::StatsRepository>,
    pub user: Arc<dyn domain::ports::UserRepository>,
    pub import_session: Arc<dyn domain::ports::ImportSessionRepository>,
    pub import_profile: Arc<dyn domain::ports::ImportProfileRepository>,
    pub movie_profile: Arc<dyn domain::ports::MovieProfileRepository>,
    pub watchlist: Arc<dyn domain::ports::WatchlistRepository>,
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub person_command: Arc<dyn domain::ports::PersonCommand>,
    pub person_query: Arc<dyn domain::ports::PersonQuery>,
    pub search_port: Arc<dyn domain::ports::SearchPort>,
    pub search_command: Arc<dyn domain::ports::SearchCommand>,
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub ap_content: Arc<dyn LocalApContentQuery>,
    pub wrapup_stats: Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn domain::ports::WrapUpRepository>,
    pub goal: Arc<dyn domain::ports::GoalRepository>,
    pub user_settings: Arc<dyn domain::ports::UserSettingsRepository>,
    pub federation_settings: std::sync::Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub remote_goal: Arc<dyn domain::ports::RemoteGoalRepository>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub db_pool: DbPool,
}

pub async fn build_database_adapters(backend: &str, url: &str) -> anyhow::Result<DatabaseOutput> {
    match backend {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let w = postgres::wire(url)
                .await
                .context("PostgreSQL connection failed")?;
            let (pc, pq) = postgres::create_person_adapter(w.pool.clone());
            let (sc, sp) = postgres_search::create_search_adapter(w.pool.clone());
            let pf = postgres::create_profile_fields_repo(w.pool.clone());
            let we = Arc::new(postgres::PostgresWatchEventRepository::new(w.pool.clone()));
            let wt: Arc<dyn WebhookTokenRepository> = Arc::new(
                postgres::PostgresWebhookTokenRepository::new(w.pool.clone()),
            );
            Ok(DatabaseOutput {
                movie_command: w.movie_command,
                movie_query: w.movie_query,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                user: w.user,
                import_session: w.import_session,
                import_profile: w.import_profile,
                movie_profile: w.movie_profile,
                watchlist: w.watchlist,
                watch_event_command: we.clone() as _,
                watch_event_query: we as _,
                webhook_token: wt,
                person_command: pc,
                person_query: pq,
                search_port: sp,
                search_command: sc,
                profile_fields: pf,
                ap_content: w.ap_content,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                goal: w.goal,
                user_settings: w.user_settings,
                federation_settings: w.federation_settings,
                remote_goal: w.remote_goal,
                refresh_session: Arc::new(postgres::PostgresRefreshSessionAdapter::new(
                    w.pool.clone(),
                )) as _,
                db_pool: DbPool::Postgres(w.pool),
            })
        }
        #[cfg(feature = "sqlite")]
        _ => {
            let w = sqlite::wire(url)
                .await
                .context("SQLite connection failed")?;
            let (pc, pq) = sqlite::create_person_adapter(w.pool.clone());
            let (sc, sp) = sqlite_search::create_search_adapter(w.pool.clone());
            let pf = sqlite::create_profile_fields_repo(w.pool.clone());
            let we = Arc::new(sqlite::SqliteWatchEventRepository::new(w.pool.clone()));
            let wt: Arc<dyn WebhookTokenRepository> =
                Arc::new(sqlite::SqliteWebhookTokenRepository::new(w.pool.clone()));
            Ok(DatabaseOutput {
                movie_command: w.movie_command,
                movie_query: w.movie_query,
                review: w.review,
                diary: w.diary,
                stats: w.stats,
                user: w.user,
                import_session: w.import_session,
                import_profile: w.import_profile,
                movie_profile: w.movie_profile,
                watchlist: w.watchlist,
                watch_event_command: we.clone() as _,
                watch_event_query: we as _,
                webhook_token: wt,
                person_command: pc,
                person_query: pq,
                search_port: sp,
                search_command: sc,
                profile_fields: pf,
                ap_content: w.ap_content,
                wrapup_stats: w.wrapup_stats,
                wrapup_repo: w.wrapup_repo,
                goal: w.goal,
                user_settings: w.user_settings,
                federation_settings: w.federation_settings,
                remote_goal: w.remote_goal,
                refresh_session: Arc::new(sqlite::SqliteRefreshSessionAdapter::new(w.pool.clone()))
                    as _,
                db_pool: DbPool::Sqlite(w.pool),
            })
        }
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!(
            "DATABASE_BACKEND={backend} is not supported by this build (enable sqlite or postgres feature)"
        ),
    }
}

pub fn build_auth_adapters() -> anyhow::Result<(Arc<dyn AuthService>, Arc<dyn PasswordHasher>)> {
    auth::create()
}

pub fn build_metadata_client() -> anyhow::Result<Arc<dyn MetadataClient>> {
    metadata::create()
}

pub fn build_poster_fetcher() -> anyhow::Result<Arc<dyn PosterFetcherClient>> {
    poster_fetcher::create()
}

pub fn build_object_storage() -> anyhow::Result<Arc<dyn ObjectStorage>> {
    object_storage::create()
}

pub fn build_profile_fields_repo(
    pool: &DbPool,
) -> anyhow::Result<Arc<dyn UserProfileFieldsRepository>> {
    match pool {
        #[cfg(feature = "postgres")]
        DbPool::Postgres(pool) => Ok(postgres::create_profile_fields_repo(pool.clone())),
        #[cfg(feature = "sqlite")]
        DbPool::Sqlite(pool) => Ok(sqlite::create_profile_fields_repo(pool.clone())),
        #[cfg(not(feature = "sqlite"))]
        _ => anyhow::bail!("no profile fields repo for this backend"),
    }
}
