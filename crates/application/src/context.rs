use std::sync::Arc;

#[cfg(feature = "federation")]
use domain::ports::RemoteWatchlistRepository;
use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher, ImageStorage,
    ImportProfileRepository, ImportSessionRepository, MetadataClient, MovieProfileRepository,
    MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
    ReviewRepository, SearchCommand, SearchPort, SocialQueryPort, StatsRepository,
    UserProfileFieldsRepository, UserRepository, WatchlistRepository,
};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppContext {
    pub movie_repository: Arc<dyn MovieRepository>,
    pub review_repository: Arc<dyn ReviewRepository>,
    pub diary_repository: Arc<dyn DiaryRepository>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
    pub document_parser: Arc<dyn DocumentParser>,
    pub stats_repository: Arc<dyn StatsRepository>,
    pub metadata_client: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub image_storage: Arc<dyn ImageStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub user_repository: Arc<dyn UserRepository>,
    pub import_session_repository: Arc<dyn ImportSessionRepository>,
    pub import_profile_repository: Arc<dyn ImportProfileRepository>,
    pub movie_profile_repository: Arc<dyn MovieProfileRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
    pub search_port: Arc<dyn SearchPort>,
    pub search_command: Arc<dyn SearchCommand>,
    pub watchlist_repository: Arc<dyn WatchlistRepository>,
    pub profile_fields_repository: Arc<dyn UserProfileFieldsRepository>,
    #[cfg(feature = "federation")]
    pub remote_watchlist_repository: Arc<dyn RemoteWatchlistRepository>,
    #[cfg(feature = "federation")]
    pub social_query: Arc<dyn SocialQueryPort>,
    pub config: AppConfig,
}
