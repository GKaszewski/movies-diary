use std::sync::Arc;

use domain::testing::{InMemoryGoalRepository, NoopEventPublisher};
use uuid::Uuid;

use crate::goals::{
    commands::{CreateGoalCommand, DeleteGoalCommand},
    create, delete,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn deletes_existing_goal() {
    let goals = InMemoryGoalRepository::new();
    let events = NoopEventPublisher::new();

    create::execute(
        Arc::clone(&goals) as _,
        Arc::clone(&events) as _,
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
        Arc::clone(&goals) as _,
        Arc::clone(&events) as _,
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
    let result = delete::execute(
        b.goal_repo.clone(),
        b.event_publisher.clone(),
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await;

    assert!(result.is_err());
}
