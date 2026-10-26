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
    let current_count = deps.stats.count_reviews_in_year(&user_id, cmd.year).await?;
    let event = DomainEvent::GoalCreated {
        goal_id: g.id().clone(),
        user_id: user_id.clone(),
        year: cmd.year,
        target_count: cmd.target_count,
    };
    deps.goal_command.save(&g, &[event]).await?;

    Ok(GoalWithProgress {
        goal: g,
        current_count,
    })
}
