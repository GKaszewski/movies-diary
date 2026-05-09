use std::sync::Arc;

use domain::ports::{
    AuthService, DiaryRepository, EventPublisher, MetadataClient, MovieRepository,
    PasswordHasher, PosterFetcherClient, PosterStorage, ReviewRepository, StatsRepository,
    UserRepository,
};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppContext {
    pub movie_repository: Arc<dyn MovieRepository>,
    pub review_repository: Arc<dyn ReviewRepository>,
    pub diary_repository: Arc<dyn DiaryRepository>,
    pub stats_repository: Arc<dyn StatsRepository>,
    pub metadata_client: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub poster_storage: Arc<dyn PosterStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub user_repository: Arc<dyn UserRepository>,
    pub config: AppConfig,
}
