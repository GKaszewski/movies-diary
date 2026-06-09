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
    let ctx = TestContextBuilder::new()
        .with_goal(Arc::clone(&goals) as _)
        .with_event_publisher(Arc::clone(&events) as _)
        .build();

    create::execute(
        &ctx,
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
        &ctx,
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
    let ctx = TestContextBuilder::new().build();
    let result = delete::execute(
        &ctx,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await;

    assert!(result.is_err());
}
