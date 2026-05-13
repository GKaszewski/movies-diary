use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::WatchlistEntry,
    value_objects::{MovieId, UserId},
};

use crate::{
    commands::AddToWatchlistCommand,
    context::AppContext,
    movie_resolver::{MovieResolver, MovieResolverDeps},
};

pub async fn execute(ctx: &AppContext, cmd: AddToWatchlistCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let movie = if let Some(id) = cmd.input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        ctx.movie_repository
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?
    } else {
        let deps = MovieResolverDeps {
            repository: ctx.movie_repository.as_ref(),
            metadata_client: ctx.metadata_client.as_ref(),
        };
        let (movie, is_new) = MovieResolver::default_pipeline()
            .resolve(&cmd.input, &deps)
            .await?;
        if is_new {
            ctx.movie_repository.upsert_movie(&movie).await?;
            if let Some(ext_id) = movie.external_metadata_id() {
                let _ = ctx
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
    ctx.watchlist_repository.add(&entry).await?;

    let _ = ctx
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
