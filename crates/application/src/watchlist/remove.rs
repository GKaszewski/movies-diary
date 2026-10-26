use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{MovieId, UserId},
};

use crate::watchlist::commands::RemoveFromWatchlistCommand;
use crate::watchlist::deps::RemoveFromWatchlistDeps;

pub async fn execute(
    deps: &RemoveFromWatchlistDeps,
    cmd: RemoveFromWatchlistCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let movie_id = MovieId::from_uuid(cmd.movie_id);
    let event = DomainEvent::WatchlistEntryRemoved {
        user_id: user_id.clone(),
        movie_id: movie_id.clone(),
    };
    deps.watchlist.remove(&user_id, &movie_id, &[event]).await?;

    Ok(())
}
