use std::sync::Arc;

use domain::{
    errors::DomainError, models::GoalWithProgress, ports::GoalRepository, value_objects::UserId,
};

use super::queries::ListGoalsQuery;

pub async fn execute(
    goal: Arc<dyn GoalRepository>,
    query: ListGoalsQuery,
) -> Result<Vec<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let goals = goal.list_for_user(&user_id).await?;

    let mut result = Vec::with_capacity(goals.len());
    for g in goals {
        let current_count = goal.count_reviews_in_year(&user_id, g.year()).await?;
        result.push(GoalWithProgress {
            goal: g,
            current_count,
        });
    }

    Ok(result)
}

#[cfg(test)]
#[path = "tests/list.rs"]
mod tests;
