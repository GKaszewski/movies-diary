use chrono::{NaiveDate, Utc};
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::ports::WrapUpRepository;
use domain::testing::{InMemoryWrapUpRepository, InMemoryWrapUpStatsQuery, NoopEventPublisher};
use domain::value_objects::WrapUpId;

use crate::wrapup::{deps::HandleWrapUpRequestedDeps, handle_requested};

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

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
        event_publisher: NoopEventPublisher::new(),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
    };

    let result = handle_requested::execute(
        &deps,
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
    let stats = InMemoryWrapUpStatsQuery::new();
    let events = NoopEventPublisher::new();
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

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
        event_publisher: events.clone(),
        wrapup_stats: stats.clone(),
    };

    let result = handle_requested::execute(
        &deps,
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

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
        event_publisher: NoopEventPublisher::new(),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
    };

    let result = handle_requested::execute(
        &deps,
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
