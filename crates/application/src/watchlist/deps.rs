use std::sync::Arc;

use domain::ports::{EventPublisher, MetadataClient, MovieRepository, WatchlistRepository};

pub struct WatchlistAddDeps {
    pub movie: Arc<dyn MovieRepository>,
    pub metadata: Arc<dyn MetadataClient>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
