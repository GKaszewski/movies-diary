use std::sync::Arc;

use domain::ports::{
    AuthService, EventPublisher, MetadataClient, MovieRepository, PasswordHasher,
    PosterFetcherClient, PosterStorage,
};

#[derive(Clone)]
pub struct AppContext {
    pub repository: Arc<dyn MovieRepository>,
    pub metadata_client: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub poster_storage: Arc<dyn PosterStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub auth_service: Arc<dyn AuthService>,
    pub password_hasher: Arc<dyn PasswordHasher>,
}
