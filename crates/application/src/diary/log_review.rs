use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, Review},
    value_objects::{Comment, MovieId, Rating, UserId},
};

use crate::{
    context::AppContext,
    diary::commands::LogReviewCommand,
    diary::movie_resolver::{MovieResolver, MovieResolverDeps},
};

pub async fn execute(ctx: &AppContext, cmd: LogReviewCommand) -> Result<(), DomainError> {
    let rating = Rating::new(cmd.rating)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    let comment = cmd.comment.clone().map(Comment::new).transpose()?;

    let (movie, is_new_movie) = if let Some(id) = cmd.input.movie_id {
        let movie_id = MovieId::from_uuid(id);
        let movie = ctx
            .repos
            .movie
            .get_movie_by_id(&movie_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Movie {id}")))?;
        (movie, false)
    } else {
        let deps = MovieResolverDeps {
            repository: ctx.repos.movie.as_ref(),
            metadata_client: ctx.services.metadata.as_ref(),
        };
        MovieResolver::default_pipeline()
            .resolve(&cmd.input, &deps)
            .await?
    };

    ctx.repos.movie.upsert_movie(&movie).await?;

    let review = Review::new(movie.id().clone(), user_id, rating, comment, cmd.watched_at)?;
    let review_event = ctx.repos.review.save_review(&review).await?;

    let was_on_watchlist = ctx
        .repos
        .watchlist
        .remove_if_present(review.user_id(), review.movie_id())
        .await?;
    if was_on_watchlist {
        let _ = ctx
            .services
            .event_publisher
            .publish(&DomainEvent::WatchlistEntryRemoved {
                user_id: review.user_id().clone(),
                movie_id: review.movie_id().clone(),
            })
            .await;
    }

    publish_events(ctx, &movie, is_new_movie, review_event).await?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/log_review.rs"]
mod tests;

async fn publish_events(
    ctx: &AppContext,
    movie: &Movie,
    is_new_movie: bool,
    review_event: DomainEvent,
) -> Result<(), DomainError> {
    if is_new_movie && let Some(ext_id) = movie.external_metadata_id() {
        let discovery_event = DomainEvent::MovieDiscovered {
            movie_id: movie.id().clone(),
            external_metadata_id: ext_id.clone(),
        };
        ctx.services
            .event_publisher
            .publish(&discovery_event)
            .await?;
    }

    if let Some(ext_id) = movie.external_metadata_id() {
        let enrichment_event = DomainEvent::MovieEnrichmentRequested {
            movie_id: movie.id().clone(),
            external_metadata_id: ext_id.value().to_string(),
        };
        ctx.services
            .event_publisher
            .publish(&enrichment_event)
            .await?;
    }

    ctx.services.event_publisher.publish(&review_event).await?;
    Ok(())
}
