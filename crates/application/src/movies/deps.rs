use std::sync::Arc;

use domain::ports::{
    EventPublisher, MetadataClient, MovieCommand, MovieProfileRepository, MovieQuery,
    ObjectStorage, PersonCommand, PersonQuery, PosterFetcherClient, SearchCommand,
};

pub struct SyncPosterDeps {
    pub movie_command: Arc<dyn MovieCommand>,
    pub movie_query: Arc<dyn MovieQuery>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub metadata: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub search_command: Arc<dyn SearchCommand>,
}

pub struct EnrichMovieDeps {
    pub movie_query: Arc<dyn MovieQuery>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub search_command: Arc<dyn SearchCommand>,
}

pub struct ReindexSearchDeps {
    pub movie_query: Arc<dyn MovieQuery>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub search_command: Arc<dyn SearchCommand>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
}
