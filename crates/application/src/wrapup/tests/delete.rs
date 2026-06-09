use std::sync::Arc;

use chrono::NaiveDate;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::testing::InMemoryWrapUpRepository;
use domain::value_objects::WrapUpId;

use crate::test_helpers::TestContextBuilder;
use crate::wrapup::delete;

#[tokio::test]
async fn deletes_existing_wrapup() {
    let repo = InMemoryWrapUpRepository::new();
    let id = WrapUpId::generate();
    repo.store.lock().unwrap().push(WrapUpRecord {
        id: id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        status: WrapUpStatus::Ready,
        report: None,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    });

    let ctx = TestContextBuilder::new().build();
    let ctx = crate::context::AppContext {
        repos: crate::context::Repositories {
            wrapup_repo: Arc::clone(&repo) as _,
            ..ctx.repos
        },
        ..ctx
    };

    delete::execute(&ctx, id).await.unwrap();
    assert_eq!(repo.store.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn fails_when_not_found() {
    let ctx = TestContextBuilder::new().build();
    let result = delete::execute(&ctx, WrapUpId::generate()).await;
    assert!(result.is_err());
}
