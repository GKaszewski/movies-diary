use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher,
    ImageStorage,
    ImportProfileRepository, ImportSessionRepository,
    MetadataClient, MovieProfileRepository, MovieRepository, PasswordHasher, PosterFetcherClient,
    PersonCommand, PersonQuery, SearchCommand, SearchPort,
    ReviewRepository, StatsRepository, UserProfileFieldsRepository, UserRepository,
    WatchlistRepository,
};
#[cfg(feature = "federation")]
use domain::ports::RemoteWatchlistRepository;

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
    pub config: AppConfig,
}
