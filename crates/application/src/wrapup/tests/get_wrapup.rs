use chrono::NaiveDate;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::testing::InMemoryWrapUpRepository;
use domain::value_objects::WrapUpId;

use crate::wrapup::get_wrapup;

#[tokio::test]
async fn returns_record_when_exists() {
    let repo = InMemoryWrapUpRepository::new();
    let id = WrapUpId::generate();
    repo.store.lock().unwrap().push(WrapUpRecord {
        id: id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    });

    let result = get_wrapup::execute(repo.clone(), id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().status, WrapUpStatus::Pending);
}

#[tokio::test]
async fn returns_none_when_missing() {
    let repo = InMemoryWrapUpRepository::new();
    let result = get_wrapup::execute(repo.clone(), WrapUpId::generate())
        .await
        .unwrap();
    assert!(result.is_none());
}
