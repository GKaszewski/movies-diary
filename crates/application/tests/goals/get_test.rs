use crate::support::create_goal_command;
use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use application::goals::{create, get, queries::GetGoalQuery};

#[tokio::test]
async fn returns_goal_when_exists() {
    let b = TestContextBuilder::new();
    let cmd_deps = GoalCommandDeps {
        goal_command: b.goal_command.clone(),
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };
    let query_deps = GoalQueryDeps {
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };

    create::execute(&cmd_deps, create_goal_command(2025, 50))
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
        goal_query: b.goal_query.clone(),
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
