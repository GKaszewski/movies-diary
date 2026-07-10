use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::DeleteGoalCommand, deps::GoalCommandDeps};

pub async fn execute(deps: &GoalCommandDeps, cmd: DeleteGoalCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let g = deps
        .goal_query
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    deps.goal_command.delete(g.id(), &user_id).await?;

    deps.event_publisher
        .publish(&DomainEvent::GoalDeleted {
            goal_id: g.id().clone(),
            user_id,
            year: cmd.year,
        })
        .await?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/delete.rs"]
mod tests;
