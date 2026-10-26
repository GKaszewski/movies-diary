use chrono::{NaiveDate, Utc};
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::ports::WrapUpRepository;
use domain::testing::{InMemoryWrapUpRepository, InMemoryWrapUpStatsQuery, RefusingWrapUpWrites};
use domain::value_objects::WrapUpId;

use application::wrapup::{deps::HandleWrapUpRequestedDeps, handle_requested};

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
    repo.create(&record, &[]).await.unwrap();

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
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
    repo.create(&record, &[]).await.unwrap();

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
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

    let recorded = repo.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        &recorded[0],
        domain::events::DomainEvent::WrapUpCompleted { wrapup_id: id } if *id == wrapup_id
    ));
}

#[tokio::test]
async fn a_wrapup_left_generating_is_completed_on_redelivery() {
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
    repo.create(&record, &[]).await.unwrap();

    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
    };

    handle_requested::execute(
        &deps,
        wrapup_id.clone(),
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    )
    .await
    .unwrap();

    let final_rec = repo.get_by_id(&wrapup_id).await.unwrap().unwrap();
    assert_eq!(final_rec.status, WrapUpStatus::Ready);
    assert!(final_rec.report.is_some());
    assert_eq!(repo.recorded_events().len(), 1);
}

#[tokio::test]
async fn wrapup_is_idempotent_when_run_twice() {
    let repo = InMemoryWrapUpRepository::new();
    let wrapup_id = WrapUpId::generate();
    let record = WrapUpRecord {
        id: wrapup_id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    repo.create(&record, &[]).await.unwrap();
    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: repo.clone(),
        wrapup_stats: InMemoryWrapUpStatsQuery::new(),
    };

    for _ in 0..2 {
        handle_requested::execute(
            &deps,
            wrapup_id.clone(),
            None,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        )
        .await
        .unwrap();
    }

    let rec = repo.get_by_id(&wrapup_id).await.unwrap().unwrap();
    assert_eq!(rec.status, WrapUpStatus::Ready);
    assert_eq!(
        repo.recorded_events().len(),
        1,
        "WrapUpCompleted emitted once"
    );
}

#[tokio::test]
async fn a_refused_completion_propagates_and_records_nothing() {
    let inner = InMemoryWrapUpRepository::new();
    let wrapup_id = WrapUpId::generate();
    let record = WrapUpRecord {
        id: wrapup_id.clone(),
        user_id: None,
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        status: WrapUpStatus::Pending,
        report: None,
        error_message: None,
        created_at: Utc::now().naive_utc(),
        completed_at: None,
    };
    inner.create(&record, &[]).await.unwrap();
    let deps = HandleWrapUpRequestedDeps {
        wrapup_repo: RefusingWrapUpWrites::new(inner.clone()),
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

    assert!(result.is_err());
    let rec = inner.get_by_id(&wrapup_id).await.unwrap().unwrap();
    assert_ne!(rec.status, WrapUpStatus::Ready);
    assert!(rec.report.is_none());
    assert!(inner.recorded_events().is_empty());
}
