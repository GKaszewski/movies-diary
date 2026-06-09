use std::sync::Arc;

use chrono::NaiveDate;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::testing::InMemoryWrapUpRepository;
use domain::value_objects::WrapUpId;
use uuid::Uuid;

use crate::test_helpers::TestContextBuilder;
use crate::wrapup::list_wrapups::{self, ListWrapUpsQuery};

fn make_record(user_id: Option<Uuid>) -> WrapUpRecord {
    WrapUpRecord {
        id: WrapUpId::generate(),
        user_id,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        status: WrapUpStatus::Ready,
        report: None,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    }
}

#[tokio::test]
async fn filters_by_user() {
    let repo = InMemoryWrapUpRepository::new();
    let uid = Uuid::new_v4();
    {
        let mut store = repo.store.lock().unwrap();
        store.push(make_record(Some(uid)));
        store.push(make_record(Some(Uuid::new_v4())));
        store.push(make_record(None));
    }

    let ctx = TestContextBuilder::new().build();
    let ctx = crate::context::AppContext {
        repos: crate::context::Repositories {
            wrapup_repo: Arc::clone(&repo) as _,
            ..ctx.repos
        },
        ..ctx
    };

    let result = list_wrapups::execute(&ctx, ListWrapUpsQuery { user_id: Some(uid) })
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].user_id, Some(uid));
}

#[tokio::test]
async fn returns_global_when_no_user() {
    let repo = InMemoryWrapUpRepository::new();
    {
        let mut store = repo.store.lock().unwrap();
        store.push(make_record(None));
        store.push(make_record(None));
        store.push(make_record(Some(Uuid::new_v4())));
    }

    let ctx = TestContextBuilder::new().build();
    let ctx = crate::context::AppContext {
        repos: crate::context::Repositories {
            wrapup_repo: Arc::clone(&repo) as _,
            ..ctx.repos
        },
        ..ctx
    };

    let result = list_wrapups::execute(&ctx, ListWrapUpsQuery { user_id: None })
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}
