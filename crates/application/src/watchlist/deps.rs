use std::sync::Arc;

use domain::ports::{
    EventPublisher, MetadataClient, MovieCommand, MovieQuery, WatchlistRepository,
};

pub struct WatchlistAddDeps {
    pub movie_command: Arc<dyn MovieCommand>,
    pub movie_query: Arc<dyn MovieQuery>,
    pub metadata: Arc<dyn MetadataClient>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
