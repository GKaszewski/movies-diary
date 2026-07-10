use std::sync::Arc;

use domain::events::DomainEvent;
use domain::testing::{FakeStatsRepository, InMemoryGoalRepository, NoopEventPublisher};
use uuid::Uuid;

use crate::goals::deps::GoalCommandDeps;
use crate::goals::{commands::CreateGoalCommand, create};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn creates_goal_and_returns_progress() {
    let goals = InMemoryGoalRepository::new();
    let stats = FakeStatsRepository::new();
    let events = NoopEventPublisher::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
        event_publisher: Arc::clone(&events) as _,
    };

    let result = create::execute(
        &deps,
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
    assert_eq!(result.current_count, 0);
    assert_eq!(goals.count(), 1);
}

#[tokio::test]
async fn creates_goal_with_review_count() {
    let goals = InMemoryGoalRepository::new();
    let stats = FakeStatsRepository::new();
    stats.set_review_count(Uuid::nil(), 2025, 5);
    let events = NoopEventPublisher::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
        event_publisher: Arc::clone(&events) as _,
    };

    let result = create::execute(
        &deps,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 50,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.current_count, 5);
    assert_eq!(goals.count(), 1);
}

#[tokio::test]
async fn emits_goal_created_event() {
    let b = TestContextBuilder::new();
    let events = NoopEventPublisher::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
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

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::GoalCreated { year: 2025, .. }))
    );
}

#[tokio::test]
async fn rejects_duplicate_year() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let cmd = CreateGoalCommand {
        user_id: Uuid::nil(),
        year: 2025,
        target_count: 10,
    };

    create::execute(&deps, cmd).await.unwrap();

    let result = create::execute(
        &deps,
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
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let result = create::execute(
        &deps,
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
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let result = create::execute(
        &deps,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 0,
        },
    )
    .await;

    assert!(result.is_err());
}
