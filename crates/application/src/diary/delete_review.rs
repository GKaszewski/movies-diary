use crate::diary::{commands::DeleteReviewCommand, deps::DeleteReviewDeps};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{ReviewId, UserId},
};

pub async fn execute(deps: &DeleteReviewDeps, cmd: DeleteReviewCommand) -> Result<(), DomainError> {
    let review_id = ReviewId::from_uuid(cmd.review_id);
    let requesting_user_id = UserId::from_uuid(cmd.requesting_user_id);

    let review = deps
        .review
        .get_review_by_id(&review_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("review {}", cmd.review_id)))?;

    if review.user_id() != &requesting_user_id {
        return Err(DomainError::Forbidden("not your review".into()));
    }

    let movie_id = review.movie_id().clone();
    let poster_path = deps
        .diary
        .get_review_history(&movie_id)
        .await?
        .movie()
        .poster_path()
        .cloned();

    deps.review
        .delete_review_and_orphaned_movie(
            &review_id,
            &movie_id,
            &[DomainEvent::ReviewDeleted {
                review_id: review_id.clone(),
                user_id: requesting_user_id,
            }],
            &[DomainEvent::MovieDeleted {
                movie_id: movie_id.clone(),
                poster_path,
            }],
        )
        .await
}
