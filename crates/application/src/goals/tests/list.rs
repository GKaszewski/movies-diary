use uuid::Uuid;

use crate::goals::{commands::CreateGoalCommand, create, list, queries::ListGoalsQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_empty_when_no_goals() {
    let ctx = TestContextBuilder::new().build();
    let result = list::execute(
        &ctx,
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
    let ctx = TestContextBuilder::new().build();
    for year in [2023, 2024, 2025] {
        create::execute(
            &ctx,
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
        &ctx,
        ListGoalsQuery {
            user_id: Uuid::nil(),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.len(), 3);
}
