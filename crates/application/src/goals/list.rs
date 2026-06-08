use domain::{errors::DomainError, models::GoalWithProgress, value_objects::UserId};

use super::queries::ListGoalsQuery;
use crate::context::AppContext;

pub async fn execute(
    ctx: &AppContext,
    query: ListGoalsQuery,
) -> Result<Vec<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let goals = ctx.repos.goal.list_for_user(&user_id).await?;

    let mut result = Vec::with_capacity(goals.len());
    for goal in goals {
        let current_count = ctx
            .repos
            .goal
            .count_reviews_in_year(&user_id, goal.year())
            .await?;
        result.push(GoalWithProgress {
            goal,
            current_count,
        });
    }

    Ok(result)
}
