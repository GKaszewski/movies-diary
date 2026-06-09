use uuid::Uuid;

use crate::goals::{
    commands::{CreateGoalCommand, UpdateGoalCommand},
    create, update,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn updates_target_count() {
    let ctx = TestContextBuilder::new().build();
    create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await
    .unwrap();

    let result = update::execute(
        &ctx,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 100,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.goal.target_count(), 100);
}

#[tokio::test]
async fn fails_when_goal_not_found() {
    let ctx = TestContextBuilder::new().build();
    let result = update::execute(
        &ctx,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_zero_target() {
    let ctx = TestContextBuilder::new().build();
    create::execute(
        &ctx,
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 10,
        },
    )
    .await
    .unwrap();

    let result = update::execute(
        &ctx,
        UpdateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 0,
        },
    )
    .await;

    assert!(result.is_err());
}
