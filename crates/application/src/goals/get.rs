use std::sync::Arc;

use domain::{
    errors::DomainError, models::GoalWithProgress, ports::GoalRepository, value_objects::UserId,
};

use super::queries::GetGoalQuery;

pub async fn execute(
    goal: Arc<dyn GoalRepository>,
    query: GetGoalQuery,
) -> Result<Option<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);

    let found = goal.find_by_user_and_year(&user_id, query.year).await?;

    let Some(g) = found else { return Ok(None) };

    let current_count = goal.count_reviews_in_year(&user_id, query.year).await?;

    Ok(Some(GoalWithProgress {
        goal: g,
        current_count,
    }))
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
