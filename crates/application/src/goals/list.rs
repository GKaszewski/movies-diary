use domain::{errors::DomainError, models::GoalWithProgress, value_objects::UserId};

use super::{deps::GoalQueryDeps, queries::ListGoalsQuery};

pub async fn execute(
    deps: &GoalQueryDeps,
    query: ListGoalsQuery,
) -> Result<Vec<GoalWithProgress>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let goals = deps.goal.list_for_user(&user_id).await?;

    let mut result = Vec::with_capacity(goals.len());
    for g in goals {
        let current_count = deps.stats.count_reviews_in_year(&user_id, g.year()).await?;
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
