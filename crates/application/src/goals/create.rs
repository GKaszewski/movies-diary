use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{Goal, GoalType, GoalWithProgress},
    value_objects::UserId,
};

use super::{commands::CreateGoalCommand, deps::GoalCommandDeps};

pub async fn execute(
    deps: &GoalCommandDeps,
    cmd: CreateGoalCommand,
) -> Result<GoalWithProgress, DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let existing = deps
        .goal_query
        .find_by_user_and_year(&user_id, cmd.year)
        .await?;
    if existing.is_some() {
        return Err(DomainError::ValidationError(
            "Goal already exists for this year".into(),
        ));
    }

    let g = Goal::new(
        user_id.clone(),
        cmd.year,
        cmd.target_count,
        GoalType::Movies,
    )?;
    deps.goal_command.save(&g).await?;

    let current_count = deps.stats.count_reviews_in_year(&user_id, cmd.year).await?;

    deps.event_publisher
        .publish(&DomainEvent::GoalCreated {
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
#[path = "tests/create.rs"]
mod tests;
