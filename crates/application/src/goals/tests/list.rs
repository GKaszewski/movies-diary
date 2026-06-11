use uuid::Uuid;

use crate::goals::{commands::CreateGoalCommand, create, list, queries::ListGoalsQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_when_no_goals() {
    let b = TestContextBuilder::new();
    let result = list::execute(
        b.goal_repo.clone(),
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
    for year in [2023, 2024, 2025] {
        create::execute(
            b.goal_repo.clone(),
            b.event_publisher.clone(),
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
        b.goal_repo.clone(),
        ListGoalsQuery {
            user_id: Uuid::nil(),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.len(), 3);
}
