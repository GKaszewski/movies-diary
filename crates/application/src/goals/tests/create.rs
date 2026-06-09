use std::sync::Arc;

use domain::events::DomainEvent;
use domain::testing::{InMemoryGoalRepository, NoopEventPublisher};
use uuid::Uuid;

use crate::goals::{commands::CreateGoalCommand, create};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn creates_goal_and_returns_progress() {
    let goals = InMemoryGoalRepository::new();
    goals.set_review_count(Uuid::nil(), 2025, 5);
    let events = NoopEventPublisher::new();
    let ctx = TestContextBuilder::new()
        .with_goal(Arc::clone(&goals) as _)
        .with_event_publisher(Arc::clone(&events) as _)
        .build();

    let result = create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 50,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.goal.year(), 2025);
    assert_eq!(result.goal.target_count(), 50);
    assert_eq!(result.current_count, 5);
    assert_eq!(goals.count(), 1);
}

#[tokio::test]
async fn emits_goal_created_event() {
    let events = NoopEventPublisher::new();
    let ctx = TestContextBuilder::new()
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

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::GoalCreated { year: 2025, .. }))
    );
}

#[tokio::test]
async fn rejects_duplicate_year() {
    let ctx = TestContextBuilder::new().build();
    let cmd = CreateGoalCommand {
        user_id: Uuid::nil(),
        year: 2025,
        target_count: 10,
    };

    create::execute(&ctx, cmd).await.unwrap();

    let result = create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 20,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_year_before_2020() {
    let ctx = TestContextBuilder::new().build();
    let result = create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2019,
            target_count: 10,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_zero_target() {
    let ctx = TestContextBuilder::new().build();
    let result = create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 0,
        },
    )
    .await;

    assert!(result.is_err());
}
