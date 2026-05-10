use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher,
    ImportProfileRepository, ImportSessionRepository,
    MetadataClient, MovieRepository, PasswordHasher, PosterFetcherClient,
    PosterStorage, ReviewRepository, StatsRepository, UserRepository,
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
    pub poster_storage: Arc<dyn PosterStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub user_repository: Arc<dyn UserRepository>,
    pub import_session_repository: Arc<dyn ImportSessionRepository>,
    pub import_profile_repository: Arc<dyn ImportProfileRepository>,
    pub config: AppConfig,
}
