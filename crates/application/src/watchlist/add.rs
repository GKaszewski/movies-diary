use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::WatchlistEntry,
    value_objects::{MovieId, UserId},
};

use crate::{
    context::AppContext,
    diary::movie_resolver::{MovieResolver, MovieResolverDeps},
    watchlist::commands::AddToWatchlistCommand,
};

pub async fn execute(ctx: &AppContext, cmd: AddToWatchlistCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let movie = if let Some(id) = cmd.input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        ctx.repos
            .movie
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?
    } else {
        let deps = MovieResolverDeps {
            repository: ctx.repos.movie.as_ref(),
            metadata_client: ctx.services.metadata.as_ref(),
        };
        let (movie, is_new) = MovieResolver::default_pipeline()
            .resolve(&cmd.input, &deps)
            .await?;
        if is_new {
            ctx.repos.movie.upsert_movie(&movie).await?;
            if let Some(ext_id) = movie.external_metadata_id() {
                let _ = ctx
                    .services
                    .event_publisher
                    .publish(&DomainEvent::MovieDiscovered {
                        movie_id: movie.id().clone(),
                        external_metadata_id: ext_id.clone(),
                    })
                    .await;
            }
        }
        movie
    };

    let entry = WatchlistEntry::new(user_id.clone(), movie.id().clone());
    ctx.repos.watchlist.add(&entry).await?;

    let _ = ctx
        .services
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
