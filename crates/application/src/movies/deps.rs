use std::sync::Arc;

use domain::ports::{
    EventPublisher, MetadataClient, MovieProfileRepository, MovieRepository, ObjectStorage,
    PersonCommand, PersonQuery, PosterFetcherClient, SearchCommand,
};

pub struct GetMoviesDeps {
    pub movie: Arc<dyn MovieRepository>,
}

pub struct GetMovieProfileDeps {
    pub movie_profile: Arc<dyn MovieProfileRepository>,
}

pub struct SyncPosterDeps {
    pub movie: Arc<dyn MovieRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub metadata: Arc<dyn MetadataClient>,
    pub poster_fetcher: Arc<dyn PosterFetcherClient>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub search_command: Arc<dyn SearchCommand>,
}

pub struct EnrichMovieDeps {
    pub movie: Arc<dyn MovieRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub person_command: Arc<dyn PersonCommand>,
    pub search_command: Arc<dyn SearchCommand>,
}

pub struct ReindexSearchDeps {
    pub movie: Arc<dyn MovieRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
    pub search_command: Arc<dyn SearchCommand>,
    pub person_command: Arc<dyn PersonCommand>,
    pub person_query: Arc<dyn PersonQuery>,
}
