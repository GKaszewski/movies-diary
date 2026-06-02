use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{MovieId, UserId},
};

use crate::{context::AppContext, watchlist::commands::RemoveFromWatchlistCommand};

pub async fn execute(ctx: &AppContext, cmd: RemoveFromWatchlistCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let movie_id = MovieId::from_uuid(cmd.movie_id);
    ctx.repos.watchlist.remove(&user_id, &movie_id).await?;

    let _ = ctx
        .services
        .event_publisher
        .publish(&DomainEvent::WatchlistEntryRemoved { user_id, movie_id })
        .await;

    Ok(())
}
