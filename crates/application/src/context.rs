use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher, ImageStorage,
    ImportProfileRepository, ImportSessionRepository, MetadataClient, MovieProfileRepository,
    MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
    RemoteWatchlistRepository, ReviewRepository, SearchCommand, SearchPort, SocialQueryPort,
    StatsRepository, UserProfileFieldsRepository, UserRepository, WatchEventRepository,
    WatchlistRepository, WebhookTokenRepository, WrapUpRepository, WrapUpStatsQuery,
    WrapUpVideoRenderer,
};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct Repositories {
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
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub remote_watchlist: Arc<dyn RemoteWatchlistRepository>,
    pub social_query: Arc<dyn SocialQueryPort>,
    pub wrapup_stats: Arc<dyn WrapUpStatsQuery>,
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}

#[derive(Clone)]
pub struct Services {
    pub auth: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub metadata: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub image_storage: Arc<dyn ImageStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub video_renderer: Option<Arc<dyn WrapUpVideoRenderer>>,
}

#[derive(Clone)]
pub struct AppContext {
    pub repos: Repositories,
    pub services: Services,
    pub config: AppConfig,
}
