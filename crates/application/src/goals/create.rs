use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Goal, GoalType, GoalWithProgress},
    value_objects::UserId,
};

use super::commands::CreateGoalCommand;
use crate::context::AppContext;

pub async fn execute(
    ctx: &AppContext,
    cmd: CreateGoalCommand,
) -> Result<GoalWithProgress, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let existing = ctx
        .repos
        .goal
        .find_by_user_and_year(&user_id, cmd.year)
        .await?;
    if existing.is_some() {
        return Err(DomainError::ValidationError(
            "Goal already exists for this year".into(),
        ));
    }

    let goal = Goal::new(
        user_id.clone(),
        cmd.year,
        cmd.target_count,
        GoalType::Movies,
    )?;
    ctx.repos.goal.save(&goal).await?;

    let current_count = ctx
        .repos
        .goal
        .count_reviews_in_year(&user_id, cmd.year)
        .await?;

    ctx.services
        .event_publisher
        .publish(&DomainEvent::GoalCreated {
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

#[cfg(test)]
#[path = "tests/create.rs"]
mod tests;
