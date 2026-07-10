use uuid::Uuid;

use crate::goals::deps::{GoalCommandDeps, GoalQueryDeps};
use crate::goals::{commands::CreateGoalCommand, create, list, queries::ListGoalsQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_when_no_goals() {
    let b = TestContextBuilder::new();
    let query_deps = GoalQueryDeps {
        goal: b.goal_repo.clone(),
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
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let query_deps = GoalQueryDeps {
        goal: b.goal_repo.clone(),
        stats: b.stats_repo.clone(),
    };

    for year in [2023, 2024, 2025] {
        create::execute(
            &cmd_deps,
            CreateGoalCommand {
                user_id: Uuid::nil(),
                year,
                target_count: 10,
            },
        )
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
