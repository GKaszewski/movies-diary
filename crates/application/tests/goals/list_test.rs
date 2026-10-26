use crate::support::create_goal_command;
use uuid::Uuid;

use crate::support::TestContextBuilder;
use application::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use application::goals::{create, list, queries::ListGoalsQuery};

#[tokio::test]
async fn returns_empty_when_no_goals() {
    let b = TestContextBuilder::new();
    let query_deps = GoalQueryDeps {
        goal_query: b.goal_query.clone(),
        stats: b.stats_repo.clone(),
    };
    let result = list::execute(
        &query_deps,
        ListGoalsQuery {
            user_id: Uuid::nil(),
        },
    )
    .await
    .unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn returns_all_goals_for_user() {
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

    for year in [2023, 2024, 2025] {
        create::execute(&cmd_deps, create_goal_command(year, 10))
            .await
            .unwrap();
    }

    let result = list::execute(
        &query_deps,
        ListGoalsQuery {
            user_id: Uuid::nil(),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.len(), 3);
}
