use crate::support::{create_goal_command, update_goal_command};

use std::sync::Arc;

use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::testing::{FakeStatsRepository, InMemoryStore, RefusingGoalCommand};

use crate::support::TestContextBuilder;
use application::goals::deps::GoalCommandDeps;
use application::goals::{create, update};

#[tokio::test]
async fn updates_target_count() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };

    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();

    let result = update::execute(&deps, update_goal_command(2025, 100))
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
    };
    let result = update::execute(&deps, update_goal_command(2025, 10)).await;

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

    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();

    let result = update::execute(&deps, update_goal_command(2025, 0)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn updating_a_goal_records_the_change_and_its_event_together() {
    let goals = InMemoryStore::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: FakeStatsRepository::new() as _,
    };
    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();

    update::execute(&deps, update_goal_command(2025, 100))
        .await
        .unwrap();

    let recorded = goals.recorded_events();
    assert_eq!(recorded.len(), 2, "created + updated");
    assert!(matches!(
        recorded[1],
        DomainEvent::GoalUpdated {
            year: 2025,
            target_count: 100,
            ..
        }
    ));
}

#[tokio::test]
async fn a_refused_update_propagates_and_records_nothing() {
    let goals = InMemoryStore::new();
    let seed = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: FakeStatsRepository::new() as _,
    };
    create::execute(&seed, create_goal_command(2025, 10))
        .await
        .unwrap();
    let before = goals.recorded_events().len();
    let deps = GoalCommandDeps {
        goal_command: Arc::new(RefusingGoalCommand),
        ..seed
    };

    let result = update::execute(&deps, update_goal_command(2025, 100)).await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
    assert_eq!(goals.recorded_events().len(), before);
}
