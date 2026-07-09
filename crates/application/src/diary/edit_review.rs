use crate::diary::{commands::EditReviewCommand, deps::EditReviewDeps};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::ReviewEdit,
    value_objects::{Comment, Rating, ReviewId, UserId},
};

pub async fn execute(deps: &EditReviewDeps, cmd: EditReviewCommand) -> Result<(), DomainError> {
    let review_id = ReviewId::from_uuid(cmd.review_id);
    let requesting_user_id = UserId::from_uuid(cmd.requesting_user_id);

    let review = deps
        .review
        .get_review_by_id(&review_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("review {}", cmd.review_id)))?;

    if review.is_remote() {
        return Err(DomainError::Forbidden(
            "cannot edit a federated review".into(),
        ));
    }

    if review.user_id() != &requesting_user_id {
        return Err(DomainError::Forbidden("not your review".into()));
    }

    let updated = review.apply_edit(ReviewEdit {
        rating: cmd.rating.map(Rating::new).transpose()?,
        comment: cmd
            .comment
            .map(|c| c.map(Comment::new).transpose())
            .transpose()?,
        watched_at: cmd.watched_at,
        watch_medium: cmd.watch_medium,
    });

    deps.review.update_review(&updated).await?;

    if let Err(e) = deps
        .event_publisher
        .publish(&DomainEvent::ReviewUpdated {
            review_id: updated.id().clone(),
            movie_id: updated.movie_id().clone(),
            user_id: updated.user_id().clone(),
            rating: updated.rating().clone(),
            watched_at: *updated.watched_at(),
        })
        .await
    {
        tracing::warn!("failed to publish ReviewUpdated: {e}");
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/edit_review.rs"]
mod tests;
