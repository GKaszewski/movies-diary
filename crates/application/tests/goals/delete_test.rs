use crate::support::create_goal_command;
use std::sync::Arc;

use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::testing::{FakeStatsRepository, InMemoryStore, RefusingGoalCommand};
use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::goals::deps::GoalCommandDeps;
use application::goals::{commands::DeleteGoalCommand, create, delete};

#[tokio::test]
async fn deletes_existing_goal() {
    let goals = InMemoryStore::new();
    let stats = FakeStatsRepository::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: Arc::clone(&stats) as _,
    };

    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();
    assert_eq!(goals.goal_count(), 1);

    delete::execute(
        &deps,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert_eq!(goals.goal_count(), 0);
}

#[tokio::test]
async fn fails_when_not_found() {
    let b = TestContextBuilder::new();
    let deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
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

#[tokio::test]
async fn deleting_a_goal_removes_it_and_records_its_event_together() {
    let goals = InMemoryStore::new();
    let deps = GoalCommandDeps {
        goal_command: Arc::clone(&goals) as _,
        goal_query: Arc::clone(&goals) as _,
        stats: FakeStatsRepository::new() as _,
    };
    create::execute(&deps, create_goal_command(2025, 10))
        .await
        .unwrap();

    delete::execute(
        &deps,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert_eq!(goals.goal_count(), 0);
    let recorded = goals.recorded_events();
    assert_eq!(recorded.len(), 2, "created + deleted");
    assert!(matches!(
        recorded[1],
        DomainEvent::GoalDeleted { year: 2025, .. }
    ));
}

#[tokio::test]
async fn a_refused_delete_propagates_and_records_nothing() {
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

    let result = delete::execute(
        &deps,
        DeleteGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(m)) if m == "refused"));
    assert_eq!(goals.goal_count(), 1);
    assert_eq!(goals.recorded_events().len(), before);
}
