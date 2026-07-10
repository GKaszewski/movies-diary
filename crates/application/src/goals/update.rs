use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::GoalWithProgress,
    value_objects::UserId,
};

use super::{commands::UpdateGoalCommand, deps::GoalCommandDeps};

pub async fn execute(
    deps: &GoalCommandDeps,
    cmd: UpdateGoalCommand,
) -> Result<GoalWithProgress, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let mut g = deps
        .goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    g.update_target(cmd.target_count)?;
    deps.goal.update(&g).await?;

    let current_count = deps.stats.count_reviews_in_year(&user_id, cmd.year).await?;

    deps.event_publisher
        .publish(&DomainEvent::GoalUpdated {
            goal_id: g.id().clone(),
            user_id,
            year: cmd.year,
            target_count: cmd.target_count,
        })
        .await?;

    Ok(GoalWithProgress {
        goal: g,
        current_count,
    })
}

#[cfg(test)]
#[path = "tests/update.rs"]
mod tests;
