use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Movie, Review},
    value_objects::{Comment, Rating, UserId},
};

use crate::{
    commands::LogReviewCommand,
    context::AppContext,
    movie_resolver::{MovieResolver, MovieResolverDeps},
};

pub async fn execute(ctx: &AppContext, cmd: LogReviewCommand) -> Result<(), DomainError> {
    let rating = Rating::new(cmd.rating)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    let comment = cmd.comment.clone().map(Comment::new).transpose()?;

    let deps = MovieResolverDeps {
        repository: ctx.movie_repository.as_ref(),
        metadata_client: ctx.metadata_client.as_ref(),
    };
    let (movie, is_new_movie) = MovieResolver::default_pipeline()
        .resolve(&cmd, &deps)
        .await?;

    ctx.movie_repository.upsert_movie(&movie).await?;

    let review = Review::new(movie.id().clone(), user_id, rating, comment, cmd.watched_at)?;
    let review_event = ctx.review_repository.save_review(&review).await?;

    publish_events(ctx, &movie, is_new_movie, review_event).await?;

    Ok(())
}

async fn publish_events(
    ctx: &AppContext,
    movie: &Movie,
    is_new_movie: bool,
    review_event: DomainEvent,
) -> Result<(), DomainError> {
    if is_new_movie {
        if let Some(ext_id) = movie.external_metadata_id() {
            let discovery_event = DomainEvent::MovieDiscovered {
                movie_id: movie.id().clone(),
                external_metadata_id: ext_id.clone(),
            };
            ctx.event_publisher.publish(&discovery_event).await?;
        }
    }

    ctx.event_publisher.publish(&review_event).await?;
    Ok(())
}
