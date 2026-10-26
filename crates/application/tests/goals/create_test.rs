use crate::support::create_goal_command;
use std::sync::Arc;

use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::testing::{FakeStatsRepository, InMemoryStore, RefusingGoalCommand};
use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::goals::create;
use application::goals::deps::GoalCommandDeps;

#[tokio::test]
async fn creates_goal_and_returns_progress() {
    let goals = InMemoryStore::new();
    let stats = FakeStatsRepository::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
    };

    let result = create::execute(&deps, create_goal_command(2025, 50))
        .await
        .unwrap();

    assert_eq!(result.goal.year(), 2025);
    assert_eq!(result.goal.target_count(), 50);
    assert_eq!(result.current_count, 0);
    assert_eq!(goals.goal_count(), 1);
}

#[tokio::test]
async fn creates_goal_with_review_count() {
    let goals = InMemoryStore::new();
    let stats = FakeStatsRepository::new();
    stats.set_review_count(Uuid::nil(), 2025, 5);
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
    };

    let result = create::execute(&deps, create_goal_command(2025, 50))
        .await
        .unwrap();

    assert_eq!(result.current_count, 5);
    assert_eq!(goals.goal_count(), 1);
}

#[tokio::test]
async fn creating_a_goal_records_the_goal_and_its_event_together() {
    let goals = InMemoryStore::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: FakeStatsRepository::new() as _,
    };

    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();

    assert_eq!(goals.goal_count(), 1);
    let recorded = goals.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        DomainEvent::GoalCreated {
            year: 2025,
            target_count: 10,
            ..
        }
    ));
}

#[tokio::test]
async fn a_refused_create_propagates_and_records_nothing() {
    let goals = InMemoryStore::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::new(RefusingGoalCommand),
        goal_query: Arc::clone(&goals) as _,
        stats: FakeStatsRepository::new() as _,
    };

    let result = create::execute(&deps, create_goal_command(2025, 10)).await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
    assert_eq!(goals.goal_count(), 0);
    assert!(goals.recorded_events().is_empty());
}

#[tokio::test]
async fn rejects_duplicate_year() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };
    let cmd = create_goal_command(2025, 10);

    create::execute(&deps, cmd).await.unwrap();

    let result = create::execute(&deps, create_goal_command(2025, 20)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_year_before_2020() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };
    let result = create::execute(&deps, create_goal_command(2019, 10)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_zero_target() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };
    let result = create::execute(&deps, create_goal_command(2025, 0)).await;

    assert!(result.is_err());
}
