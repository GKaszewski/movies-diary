use crate::{commands::DeleteReviewCommand, context::AppContext};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ReviewId, UserId},
};

pub async fn execute(ctx: &AppContext, cmd: DeleteReviewCommand) -> Result<(), DomainError> {
    let review_id = ReviewId::from_uuid(cmd.review_id);
    let requesting_user_id = UserId::from_uuid(cmd.requesting_user_id);

    let review = ctx
        .review_repository
        .get_review_by_id(&review_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("review {}", cmd.review_id)))?;

    if review.user_id() != &requesting_user_id {
        return Err(DomainError::Unauthorized("not your review".into()));
    }

    let movie_id = review.movie_id().clone();
    ctx.review_repository.delete_review(&review_id).await?;

    if let Err(e) = ctx
        .event_publisher
        .publish(&DomainEvent::ReviewDeleted {
            review_id: review_id.clone(),
            user_id: requesting_user_id.clone(),
        })
        .await
    {
        tracing::warn!("failed to publish ReviewDeleted: {e}");
    }

    let history = ctx.diary_repository.get_review_history(&movie_id).await?;
    if history.viewings().is_empty() {
        let poster_path = history.movie().poster_path().cloned();
        ctx.movie_repository.delete_movie(&movie_id).await?;
        // best-effort: movie is already deleted, so publish failure is non-fatal
        if let Err(e) = ctx.event_publisher
            .publish(&DomainEvent::MovieDeleted { movie_id, poster_path })
            .await
        {
            tracing::warn!("failed to publish MovieDeleted event: {e}");
        }
    }

    Ok(())
}
