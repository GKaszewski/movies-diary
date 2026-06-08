use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::commands::DeleteGoalCommand;
use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, cmd: DeleteGoalCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let goal = ctx
        .repos
        .goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    ctx.repos.goal.delete(goal.id(), &user_id).await?;

    ctx.services
        .event_publisher
        .publish(&DomainEvent::GoalDeleted {
            goal_id: goal.id().clone(),
            user_id,
            year: cmd.year,
        })
        .await?;

    Ok(())
}
