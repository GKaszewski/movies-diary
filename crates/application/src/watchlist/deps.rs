use std::sync::Arc;

use domain::ports::{
    EventPublisher, MetadataClient, MovieCommand, MovieQuery, RemoteWatchlistRepository,
    UserRepository, WatchlistRepository,
};

pub struct WatchlistAddDeps {
    pub movie_command: Arc<dyn MovieCommand>,
    pub movie_query: Arc<dyn MovieQuery>,
    pub metadata: Arc<dyn MetadataClient>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct GetWatchlistForOwnerDeps {
    pub user: Arc<dyn UserRepository>,
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub remote_watchlist: Arc<dyn RemoteWatchlistRepository>,
}

pub struct GetWatchlistDeps {
    pub watchlist: Arc<dyn WatchlistRepository>,
}

pub struct IsOnWatchlistDeps {
    pub watchlist: Arc<dyn WatchlistRepository>,
}

pub struct RemoveFromWatchlistDeps {
    pub watchlist: Arc<dyn WatchlistRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
