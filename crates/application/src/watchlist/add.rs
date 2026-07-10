use domain::{
    errors::DomainError, events::DomainEvent, models::WatchlistEntry, value_objects::UserId,
};

use crate::{
    movies::resolve::resolve_and_persist_movie,
    watchlist::{commands::AddToWatchlistCommand, deps::WatchlistAddDeps},
};

pub async fn execute(
    deps: &WatchlistAddDeps,
    cmd: AddToWatchlistCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let (movie, _is_new) = resolve_and_persist_movie(
        &cmd.input,
        deps.movie_command.as_ref(),
        deps.movie_query.as_ref(),
        deps.metadata.as_ref(),
        deps.event_publisher.as_ref(),
    )
    .await?;

    let entry = WatchlistEntry::new(user_id.clone(), movie.id().clone());
    deps.watchlist.add(&entry).await?;

    let _ = deps
        .event_publisher
        .publish(&DomainEvent::WatchlistEntryAdded {
            user_id,
            movie_id: movie.id().clone(),
            movie_title: movie.title().value().to_string(),
            release_year: movie.release_year().value(),
            external_metadata_id: movie.external_metadata_id().map(|e| e.value().to_string()),
            added_at: entry.added_at,
        })
        .await;

    Ok(())
}

#[cfg(test)]
#[path = "tests/add.rs"]
mod tests;
