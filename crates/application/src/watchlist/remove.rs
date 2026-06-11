use std::sync::Arc;

use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, WatchlistRepository},
    value_objects::{MovieId, UserId},
};

use crate::watchlist::commands::RemoveFromWatchlistCommand;

pub async fn execute(
    watchlist: Arc<dyn WatchlistRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    cmd: RemoveFromWatchlistCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let movie_id = MovieId::from_uuid(cmd.movie_id);
    watchlist.remove(&user_id, &movie_id).await?;

    let _ = event_publisher
        .publish(&DomainEvent::WatchlistEntryRemoved { user_id, movie_id })
        .await;

    Ok(())
}

#[cfg(test)]
#[path = "tests/remove.rs"]
mod tests;
