use std::sync::Arc;

use anyhow::Context;
use domain::ports::{
    AuthService, ImageRefCommand, ImageRefQuery, MetadataClient, MovieDeduplicator, ObjectStorage,
    PasswordHasher, PosterFetcherClient, RefreshSessionRepository, UserProfileFieldsRepository,
    WatchEventCommand, WatchEventQuery, WebhookTokenRepository,
};

pub struct DatabaseOutput {
    pub movie_command: Arc<dyn domain::ports::MovieCommand>,
    pub movie_query: Arc<dyn domain::ports::MovieQuery>,
    pub review: Arc<dyn domain::ports::ReviewRepository>,
    pub diary: Arc<dyn domain::ports::DiaryQuery>,
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
    #[cfg(feature = "federation")]
    pub ap_content: Arc<dyn domain::ports::LocalApContentQuery>,
    pub wrapup_stats: Arc<dyn domain::ports::WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn domain::ports::WrapUpRepository>,
    pub insights: Arc<dyn domain::ports::InsightsQuery>,
    pub goal_command: Arc<dyn domain::ports::GoalCommand>,
    pub goal_query: Arc<dyn domain::ports::GoalQuery>,
    pub user_settings: Arc<dyn domain::ports::UserSettingsRepository>,
    #[cfg(feature = "federation")]
    pub federation_settings: std::sync::Arc<dyn domain::ports::UserFederationSettingsQuery>,
    pub remote_goal: Arc<dyn domain::ports::RemoteGoalRepository>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub processed_events: Arc<dyn domain::ports::ProcessedEventLedger>,
    pub deduplicator: Arc<dyn MovieDeduplicator>,
    pub account_purger: Arc<dyn domain::ports::AccountPurger>,
    pub image_ref_command: Arc<dyn ImageRefCommand>,
    pub image_ref_query: Arc<dyn ImageRefQuery>,
    pub follow_command: Arc<dyn domain::ports::FollowCommand>,
    pub follow_query: Arc<dyn domain::ports::FollowQuery>,
    pub db_pool: sqlx::SqlitePool,
}

pub async fn build_database_adapters(
    url: &str,
    instance: &domain::value_objects::InstanceIdentity,
) -> anyhow::Result<DatabaseOutput> {
    let w = sqlite::wire(url)
        .await
        .context("SQLite connection failed")?;
    assemble(w, instance)
}

pub async fn build_database_adapters_on(
    pool: sqlx::SqlitePool,
    instance: &domain::value_objects::InstanceIdentity,
) -> anyhow::Result<DatabaseOutput> {
    let w = sqlite::wire_pool(pool)
        .await
        .context("SQLite wiring failed")?;
    assemble(w, instance)
}

fn assemble(
    w: sqlite::SqliteWireOutput,
    instance: &domain::value_objects::InstanceIdentity,
) -> anyhow::Result<DatabaseOutput> {
    let (image_ref_command, image_ref_query) = sqlite::create_image_ref(w.pool.clone());
    let (pc, pq) = sqlite::create_person_adapter(w.pool.clone());
    let (sc, sp) = sqlite::create_search_adapter(w.pool.clone());
    let pf = sqlite::create_profile_fields_repo(w.pool.clone());
    let we = Arc::new(sqlite::SqliteWatchEventRepository::new(w.pool.clone()));
    let wt: Arc<dyn WebhookTokenRepository> =
        Arc::new(sqlite::SqliteWebhookTokenRepository::new(w.pool.clone()));
    let social = Arc::new(sqlite::SqliteSocialRepository::new(
        w.pool.clone(),
        instance.clone(),
    ));
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
        #[cfg(feature = "federation")]
        ap_content: w.ap_content,
        wrapup_stats: w.wrapup_stats,
        wrapup_repo: w.wrapup_repo,
        insights: w.insights,
        goal_command: w.goal_command,
        goal_query: w.goal_query,
        user_settings: w.user_settings,
        #[cfg(feature = "federation")]
        federation_settings: w.federation_settings,
        remote_goal: w.remote_goal,
        refresh_session: Arc::new(sqlite::SqliteRefreshSessionAdapter::new(w.pool.clone())) as _,
        processed_events: Arc::new(sqlite::SqliteProcessedEventLedger::new(w.pool.clone())) as _,
        deduplicator: w.deduplicator,
        account_purger: Arc::new(sqlite::SqliteAccountPurger::new(
            w.pool.clone(),
            instance.clone(),
        )) as _,
        image_ref_command,
        image_ref_query,
        follow_command: Arc::clone(&social) as _,
        follow_query: social as _,
        db_pool: w.pool,
    })
}

pub fn build_auth_adapters(
    config: &config::AuthConfig,
) -> anyhow::Result<(Arc<dyn AuthService>, Arc<dyn PasswordHasher>)> {
    auth::create(config)
}

pub fn build_metadata_client(
    config: &config::MetadataConfig,
) -> anyhow::Result<Arc<dyn MetadataClient>> {
    metadata::create(config)
}

pub fn build_poster_fetcher(
    config: &config::MetadataConfig,
) -> anyhow::Result<Arc<dyn PosterFetcherClient>> {
    poster_fetcher::create(config)
}

pub fn build_object_storage(
    config: &config::StorageConfig,
) -> anyhow::Result<Arc<dyn ObjectStorage>> {
    object_storage::create(config)
}

pub fn warn_if_database_backend_set() {
    if let Ok(v) = std::env::var("DATABASE_BACKEND")
        && v != "sqlite"
    {
        tracing::warn!("DATABASE_BACKEND is ignored; SQLite is the only backend");
    }
}
