use uuid::Uuid;

use crate::goals::deps::GoalCommandDeps;
use crate::goals::{
    commands::{CreateGoalCommand, UpdateGoalCommand},
    create, update,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn updates_target_count() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };

    create::execute(
        &deps,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await
    .unwrap();

    let result = update::execute(
        &deps,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 100,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.goal.target_count(), 100);
}

#[tokio::test]
async fn fails_when_goal_not_found() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let result = update::execute(
        &deps,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_zero_target() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };

    create::execute(
        &deps,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await
    .unwrap();

    let result = update::execute(
        &deps,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 0,
        },
    )
    .await;

    assert!(result.is_err());
}
