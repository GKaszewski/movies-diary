use domain::{errors::DomainError, models::GoalWithProgress, value_objects::UserId};

use super::queries::GetGoalQuery;
use crate::context::AppContext;

pub async fn execute(
    ctx: &AppContext,
    query: GetGoalQuery,
) -> Result<Option<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);

    let goal = ctx
        .repos
        .goal
        .find_by_user_and_year(&user_id, query.year)
        .await?;

    let Some(goal) = goal else { return Ok(None) };

    let current_count = ctx
        .repos
        .goal
        .count_reviews_in_year(&user_id, query.year)
        .await?;

    Ok(Some(GoalWithProgress {
        goal,
        current_count,
    }))
}
