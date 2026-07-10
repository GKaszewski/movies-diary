use std::sync::Arc;

use domain::testing::{FakeStatsRepository, InMemoryGoalRepository, NoopEventPublisher};
use uuid::Uuid;

use crate::goals::deps::GoalCommandDeps;
use crate::goals::{
    commands::{CreateGoalCommand, DeleteGoalCommand},
    create, delete,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn deletes_existing_goal() {
    let goals = InMemoryGoalRepository::new();
    let stats = FakeStatsRepository::new();
    let events = NoopEventPublisher::new();
    let deps = GoalCommandDeps {
        goal: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
        event_publisher: Arc::clone(&events) as _,
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
    assert_eq!(goals.count(), 1);

    delete::execute(
        &deps,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert_eq!(goals.count(), 0);
}

#[tokio::test]
async fn fails_when_not_found() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let result = delete::execute(
        &deps,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await;

    assert!(result.is_err());
}
