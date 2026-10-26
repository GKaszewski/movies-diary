#![allow(dead_code)]

use application::config::AppSettings;
use bootstrap::Repositories;
use domain::{
    errors::DomainError,
    models::GeneratedToken,
    ports::{AuthService, PersonQuery, SearchPort, UserRepository},
    testing::{
        PanicAuthService, PanicDiaryExporter, PanicDiaryQuery, PanicDocumentParser,
        PanicEventPublisher, PanicGoalCommand, PanicGoalQuery, PanicImportProfileRepository,
        PanicImportSessionRepository, PanicInsightsQuery, PanicMetadataClient, PanicMovieCommand,
        PanicMovieProfileRepository, PanicMovieQuery, PanicObjectStorage, PanicPasswordHasher,
        PanicPersonCommand, PanicPersonQuery, PanicPosterFetcher, PanicProfileFieldsRepo,
        PanicRefreshSessionRepository, PanicRemoteGoalRepository, PanicRemoteWatchlistRepository,
        PanicReviewRepository, PanicRssFeedRenderer, PanicSearchCommand, PanicSearchPort,
        PanicStatsRepository, PanicUserRepository, PanicUserSettingsRepository,
        PanicWatchEventCommand, PanicWatchEventQuery, PanicWatchlistRepository,
        PanicWebhookTokenRepository, PanicWrapUpRepository, PanicWrapUpStatsQuery,
    },
    value_objects::UserId,
};
use http_axum::context::AppContext;
use std::sync::Arc;

struct PanicReviewLogger;

#[async_trait::async_trait]
impl application::ports::ReviewLogger for PanicReviewLogger {
    async fn log_review(
        &self,
        _: application::diary::commands::LogReviewCommand,
    ) -> Result<(), DomainError> {
        panic!()
    }
}

// --- Repositories factory — `Panic*` stubs everywhere ---

pub fn test_repositories() -> Repositories {
    Repositories {
        movie_command: Arc::new(PanicMovieCommand) as _,
        movie_query: Arc::new(PanicMovieQuery) as _,
        review: Arc::new(PanicReviewRepository) as _,
        diary: Arc::new(PanicDiaryQuery) as _,
        stats: Arc::new(PanicStatsRepository) as _,
        user: Arc::new(PanicUserRepository) as _,
        import_session: Arc::new(PanicImportSessionRepository) as _,
        import_profile: Arc::new(PanicImportProfileRepository) as _,
        movie_profile: Arc::new(PanicMovieProfileRepository) as _,
        watchlist: Arc::new(PanicWatchlistRepository) as _,
        watch_event_command: Arc::new(PanicWatchEventCommand) as _,
        watch_event_query: Arc::new(PanicWatchEventQuery) as _,
        webhook_token: Arc::new(PanicWebhookTokenRepository) as _,
        profile_fields: Arc::new(PanicProfileFieldsRepo) as _,
        person_command: Arc::new(PanicPersonCommand) as _,
        person_query: Arc::new(PanicPersonQuery) as _,
        search_port: Arc::new(PanicSearchPort) as _,
        search_command: Arc::new(PanicSearchCommand) as _,
        remote_watchlist: Arc::new(PanicRemoteWatchlistRepository) as _,
        social_command: Arc::new(domain::ports::noop::NoopSocialCommand) as _,
        follow_graph: Arc::new(domain::ports::noop::NoopSocialQuery) as _,
        block_query: Arc::new(domain::ports::noop::NoopSocialQuery) as _,
        federation_admin: Arc::new(domain::ports::noop::NoopFederationAdminQuery) as _,
        wrapup_stats: Arc::new(PanicWrapUpStatsQuery) as _,
        wrapup_repo: Arc::new(PanicWrapUpRepository) as _,
        insights: Arc::new(PanicInsightsQuery) as _,
        goal_command: Arc::new(PanicGoalCommand) as _,
        goal_query: Arc::new(PanicGoalQuery) as _,
        user_settings: Arc::new(PanicUserSettingsRepository) as _,
        remote_goal: Arc::new(PanicRemoteGoalRepository) as _,
        refresh_session: Arc::new(PanicRefreshSessionRepository) as _,
        federated_profile: None,
    }
}

// --- Single state factory — builds `deps` from whatever `repos` it is given ---

/// Builds an `AppState` from caller-supplied `repos`, running `bootstrap::build_deps`
/// AFTER any overrides the caller already applied to `repos`. This is the only place
/// `deps` gets built for tests, so `repos` and `deps` can never disagree.
pub fn build_test_state_from(
    repos: Repositories,
    auth_service: Arc<dyn AuthService>,
) -> http_axum::state::AppState {
    let services = application::Services {
        auth: auth_service,
        password_hasher: Arc::new(PanicPasswordHasher) as _,
        metadata: Arc::new(PanicMetadataClient) as _,
        poster_fetcher: Arc::new(PanicPosterFetcher) as _,
        object_storage: Arc::new(PanicObjectStorage) as _,
        event_publisher: Arc::new(PanicEventPublisher) as _,
        diary_exporter: Arc::new(PanicDiaryExporter) as _,
        document_parser: Arc::new(PanicDocumentParser) as _,
        review_logger: Arc::new(PanicReviewLogger) as _,
        person_enrichment: None,
    };

    let config = AppSettings {
        allow_registration: false,
        base_url: "http://localhost:3000".to_string(),
        rate_limit: 20,
        refresh_ttl_seconds: 2_592_000,
        secure_cookies: false,
        cors_origins: String::new(),
        wrapup: application::config::WrapUpSettings {
            font_path: None,
            logo_path: None,
            bg_dir: None,
        },
    };

    let instance = domain::value_objects::InstanceIdentity::new("http://localhost:3000");

    let deps = Arc::new(bootstrap::build_deps(&repos, &services, &config, &instance));

    http_axum::state::AppState {
        app_ctx: AppContext {
            deps,
            services,
            config,
            instance,
            #[cfg(feature = "federation")]
            ap_document: Arc::new(domain::ports::noop::NoopApDocument),
            #[cfg(feature = "federation")]
            ap_blocklist: Arc::new(domain::ports::noop::NoopInstanceBlocklist),
        },
        rss_renderer: Arc::new(PanicRssFeedRenderer),
    }
}

pub fn make_test_state(auth_service: Arc<dyn AuthService>) -> http_axum::state::AppState {
    build_test_state_from(test_repositories(), auth_service)
}

/// Reusable default test `AppState`: `Panic*` stubs everywhere, real prebuilt `deps`.
/// Callers that need a specific stub swapped in should use one of the
/// `test_app_state_with_*` constructors below, which inject BEFORE `build_deps`
/// runs — mutating the returned `AppState` afterwards would only reach `repos`,
/// not the prebuilt `deps`.
pub fn test_app_state() -> http_axum::state::AppState {
    make_test_state(Arc::new(PanicAuthService))
}

/// Test state with `person_query` overridden BEFORE `build_deps` runs, so the
/// stub is visible from both `app_ctx.repos.person_query` and every prebuilt
/// deps struct that carries a `person_query` (e.g. `deps.person.get_person`).
pub fn test_app_state_with_person_query(
    person_query: Arc<dyn PersonQuery>,
) -> http_axum::state::AppState {
    let mut repos = test_repositories();
    repos.person_query = person_query;
    build_test_state_from(repos, Arc::new(PanicAuthService))
}

/// Test state with `search_port` overridden BEFORE `build_deps` runs, so the
/// stub is visible from both `app_ctx.repos.search_port` and any prebuilt deps
/// struct that carries a `search_port`.
pub fn test_app_state_with_search_port(
    search_port: Arc<dyn SearchPort>,
) -> http_axum::state::AppState {
    let mut repos = test_repositories();
    repos.search_port = search_port;
    build_test_state_from(repos, Arc::new(PanicAuthService))
}

pub fn test_app_state_with_repos(
    adjust: impl FnOnce(&mut Repositories),
    auth_service: Arc<dyn AuthService>,
) -> http_axum::state::AppState {
    let mut repos = test_repositories();
    adjust(&mut repos);
    build_test_state_from(repos, auth_service)
}

/// Test state with `user` overridden BEFORE `build_deps` runs, so the stub is
/// visible from both `app_ctx.repos.user` and `deps.users.authorize_admin.user`
/// — the field the `AdminApiUser`/`AdminUser` extractors now read through.
pub fn test_app_state_with_user(
    user: Arc<dyn UserRepository>,
    auth_service: Arc<dyn AuthService>,
) -> http_axum::state::AppState {
    let mut repos = test_repositories();
    repos.user = user;
    build_test_state_from(repos, auth_service)
}

/// A `UserRepository` whose `find_by_id` always fails, for exercising the
/// repository-error rejection path of the admin extractors. Every other
/// method panics — the admin extractors only ever call `find_by_id`.
pub struct ErroringUserRepo;
#[async_trait::async_trait]
impl UserRepository for ErroringUserRepo {
    async fn find_by_email(
        &self,
        _: &domain::value_objects::Email,
    ) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn save(&self, _: &domain::models::User) -> Result<(), DomainError> {
        panic!()
    }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<domain::models::User>, DomainError> {
        Err(DomainError::InfrastructureError("db down".into()))
    }
    async fn find_by_username(
        &self,
        _: &domain::value_objects::Username,
    ) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        panic!()
    }
    async fn update_profile(
        &self,
        _: &UserId,
        _: &domain::models::UserProfile,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn mark_deleted(
        &self,
        _: &UserId,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn find_tombstoned_by_id(
        &self,
        _: &UserId,
    ) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn is_username_retired(
        &self,
        _: &domain::value_objects::Username,
    ) -> Result<bool, DomainError> {
        panic!()
    }
    async fn count_active_admins(&self) -> Result<u64, DomainError> {
        panic!()
    }
}

/// Auth stub that accepts any bearer token and resolves it to a fixed user id,
/// for tests that need to authenticate as a specific user rather than merely
/// exercise the unauthenticated-request path.
pub struct AcceptingAuth(pub UserId);
#[async_trait::async_trait]
impl AuthService for AcceptingAuth {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!()
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        Ok(self.0.clone())
    }
}
