use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{MovieId, UserId},
};

use crate::{commands::RemoveFromWatchlistCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: RemoveFromWatchlistCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let movie_id = MovieId::from_uuid(cmd.movie_id);
    ctx.watchlist_repository.remove(&user_id, &movie_id).await?;

    let _ = ctx
        .event_publisher
        .publish(&DomainEvent::WatchlistEntryRemoved { user_id, movie_id })
        .await;

    Ok(())
}
