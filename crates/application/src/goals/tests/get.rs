use uuid::Uuid;

use crate::goals::{commands::CreateGoalCommand, create, get, queries::GetGoalQuery};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn returns_goal_when_exists() {
    let b = TestContextBuilder::new();
    create::execute(
        b.goal_repo.clone(),
        b.stats_repo.clone(),
        b.event_publisher.clone(),
        CreateGoalCommand {
            user_id: Uuid::nil(),
            year: 2025,
            target_count: 50,
        },
    )
    .await
    .unwrap();

    let result = get::execute(
        b.goal_repo.clone(),
        b.stats_repo.clone(),
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
    let result = get::execute(
        b.goal_repo.clone(),
        b.stats_repo.clone(),
        GetGoalQuery {
            user_id: Uuid::nil(),
            year: 2025,
        },
    )
    .await
    .unwrap();

    assert!(result.is_none());
}
