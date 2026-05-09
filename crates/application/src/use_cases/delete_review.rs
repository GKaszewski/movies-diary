use crate::{commands::DeleteReviewCommand, context::AppContext};
use domain::{
    errors::DomainError,
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

    let history = ctx.diary_repository.get_review_history(&movie_id).await?;
    if history.viewings().is_empty() {
        ctx.movie_repository.delete_movie(&movie_id).await?;
    }

    Ok(())
}
