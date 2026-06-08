use domain::{
    errors::DomainError, events::DomainEvent, models::GoalWithProgress, value_objects::UserId,
};

use super::commands::UpdateGoalCommand;
use crate::context::AppContext;

pub async fn execute(
    ctx: &AppContext,
    cmd: UpdateGoalCommand,
) -> Result<GoalWithProgress, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let mut goal = ctx
        .repos
        .goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    goal.update_target(cmd.target_count)?;
    ctx.repos.goal.update(&goal).await?;

    let current_count = ctx
        .repos
        .goal
        .count_reviews_in_year(&user_id, cmd.year)
        .await?;

    ctx.services
        .event_publisher
        .publish(&DomainEvent::GoalUpdated {
            goal_id: goal.id().clone(),
            user_id,
            year: cmd.year,
            target_count: cmd.target_count,
        })
        .await?;

    Ok(GoalWithProgress {
        goal,
        current_count,
    })
}
