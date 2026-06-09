use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::ports::WrapUpRepository;
use domain::testing::InMemoryWrapUpRepository;
use domain::value_objects::WrapUpId;

use crate::test_helpers::TestContextBuilder;
use crate::wrapup::handle_requested;

#[tokio::test]
async fn skips_if_already_ready() {
    let repo = InMemoryWrapUpRepository::new();
    let wrapup_id = WrapUpId::generate();

    let record = WrapUpRecord {
        id: wrapup_id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Ready,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    repo.create(&record).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_wrapup_repo(Arc::clone(&repo) as _)
        .build();

    let result = handle_requested::execute(
        &ctx,
        wrapup_id,
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn generates_wrapup_and_marks_complete() {
    let repo = InMemoryWrapUpRepository::new();
    let stats = domain::testing::InMemoryWrapUpStatsQuery::new();
    let events = domain::testing::NoopEventPublisher::new();
    let wrapup_id = WrapUpId::generate();
    let uid = uuid::Uuid::new_v4();

    let record = WrapUpRecord {
        id: wrapup_id.clone(),
        user_id: Some(uid),
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    repo.create(&record).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_wrapup_repo(Arc::clone(&repo) as _)
        .wrapup_stats(Arc::clone(&stats) as _)
        .with_event_publisher(Arc::clone(&events) as _)
        .build();

    let result = handle_requested::execute(
        &ctx,
        wrapup_id.clone(),
        Some(uid),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    )
    .await;

    assert!(result.is_ok());

    // Verify it was marked as Ready
    let final_rec = repo.get_by_id(&wrapup_id).await.unwrap().unwrap();
    assert_eq!(final_rec.status, WrapUpStatus::Ready);
    assert!(final_rec.report.is_some());

    // Verify event was published
    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, domain::events::DomainEvent::WrapUpCompleted { .. }))
    );
}

#[tokio::test]
async fn skips_if_already_generating() {
    let repo = InMemoryWrapUpRepository::new();
    let wrapup_id = WrapUpId::generate();

    let record = WrapUpRecord {
        id: wrapup_id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Generating,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    repo.create(&record).await.unwrap();

    let ctx = TestContextBuilder::new()
        .with_wrapup_repo(Arc::clone(&repo) as _)
        .build();

    let result = handle_requested::execute(
        &ctx,
        wrapup_id.clone(),
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    )
    .await;

    assert!(result.is_ok());

    // Status should still be Generating (not changed to Ready)
    let final_rec = repo.get_by_id(&wrapup_id).await.unwrap().unwrap();
    assert_eq!(final_rec.status, WrapUpStatus::Generating);
}
