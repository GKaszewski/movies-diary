use uuid::Uuid;

use crate::goals::{commands::CreateGoalCommand, create, get, queries::GetGoalQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_goal_when_exists() {
    let ctx = TestContextBuilder::new().build();
    create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 50,
        },
    )
    .await
    .unwrap();

    let result = get::execute(
        &ctx,
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
    let ctx = TestContextBuilder::new().build();
    let result = get::execute(
        &ctx,
        GetGoalQuery {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert!(result.is_none());
}
