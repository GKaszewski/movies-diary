use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher,
    FederatedProfileQuery, GoalRepository, ImportProfileRepository, ImportSessionRepository,
    MetadataClient, MovieCommand, MovieProfileRepository, MovieQuery, ObjectStorage,
    PasswordHasher, PersonCommand, PersonEnrichmentClient, PersonQuery, PosterFetcherClient,
    RefreshSessionRepository, RemoteGoalRepository, RemoteWatchlistRepository, ReviewRepository,
    SearchCommand, SearchPort, SocialQueryPort, StatsRepository, UserProfileFieldsRepository,
    UserRepository, UserSettingsRepository, WatchEventCommand, WatchEventQuery,
    WatchlistRepository, WebhookTokenRepository, WrapUpRepository, WrapUpStatsQuery,
};

use application::config::AppConfig;
use application::ports::ReviewLogger;

#[derive(Clone)]
pub struct Repositories {
    pub movie_command: Arc<dyn MovieCommand>,
    pub movie_query: Arc<dyn MovieQuery>,
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub stats: Arc<dyn StatsRepository>,
    pub user: Arc<dyn UserRepository>,
    pub import_session: Arc<dyn ImportSessionRepository>,
    pub import_profile: Arc<dyn ImportProfileRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub remote_watchlist: Arc<dyn RemoteWatchlistRepository>,
    pub social_query: Arc<dyn SocialQueryPort>,
    pub wrapup_stats: Arc<dyn WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
    pub goal: Arc<dyn GoalRepository>,
    pub user_settings: Arc<dyn UserSettingsRepository>,
    pub remote_goal: Arc<dyn RemoteGoalRepository>,
    pub refresh_session: Arc<dyn RefreshSessionRepository>,
    pub federated_profile: Option<Arc<dyn FederatedProfileQuery>>,
}

#[derive(Clone)]
pub struct Services {
    pub auth: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub metadata: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub review_logger: Arc<dyn ReviewLogger>,
    pub person_enrichment: Option<Arc<dyn PersonEnrichmentClient>>,
}

#[derive(Clone)]
pub struct AppContext {
    pub repos: Repositories,
    pub services: Services,
    pub config: AppConfig,
}
