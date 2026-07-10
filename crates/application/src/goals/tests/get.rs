use uuid::Uuid;

use crate::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use crate::goals::{commands::CreateGoalCommand, create, get, queries::GetGoalQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_goal_when_exists() {
    let b = TestContextBuilder::new();
    let cmd_deps = GoalCommandDeps {
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let query_deps = GoalQueryDeps {
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
    };

    create::execute(
        &cmd_deps,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 50,
        },
    )
    .await
    .unwrap();

    let result = get::execute(
        &query_deps,
        GetGoalQuery {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().goal.target_count(), 50);
}

#[tokio::test]
async fn returns_none_when_missing() {
    let b = TestContextBuilder::new();
    let query_deps = GoalQueryDeps {
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
    };
    let result = get::execute(
        &query_deps,
        GetGoalQuery {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert!(result.is_none());
}
