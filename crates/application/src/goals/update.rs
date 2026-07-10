use std::sync::Arc;

use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::GoalWithProgress,
    ports::{EventPublisher, GoalRepository, StatsRepository},
    value_objects::UserId,
};

use super::commands::UpdateGoalCommand;

pub async fn execute(
    goal: Arc<dyn GoalRepository>,
    stats: Arc<dyn StatsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    cmd: UpdateGoalCommand,
) -> Result<GoalWithProgress, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let mut g = goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Goal for year {}", cmd.year)))?;

    g.update_target(cmd.target_count)?;
    goal.update(&g).await?;

    let current_count = stats.count_reviews_in_year(&user_id, cmd.year).await?;

    event_publisher
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
