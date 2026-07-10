use domain::{errors::DomainError, models::GoalWithProgress, value_objects::UserId};

use super::{deps::GoalQueryDeps, queries::GetGoalQuery};

pub async fn execute(
    deps: &GoalQueryDeps,
    query: GetGoalQuery,
) -> Result<Option<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);

    let found = deps
        .goal
        .find_by_user_and_year(&user_id, query.year)
        .await?;

    let Some(g) = found else { return Ok(None) };

    let current_count = deps
        .stats
        .count_reviews_in_year(&user_id, query.year)
        .await?;

    Ok(Some(GoalWithProgress {
        goal: g,
        current_count,
    }))
}

#[cfg(test)]
#[path = "tests/get.rs"]
mod tests;
