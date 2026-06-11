use std::sync::Arc;

use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, GoalRepository},
    value_objects::UserId,
};

use super::commands::DeleteGoalCommand;

pub async fn execute(
    goal: Arc<dyn GoalRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    cmd: DeleteGoalCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let g = goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    goal.delete(g.id(), &user_id).await?;

    event_publisher
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
