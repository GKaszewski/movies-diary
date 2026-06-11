use chrono::NaiveDate;
use domain::events::DomainEvent;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::testing::{InMemoryWrapUpRepository, NoopEventPublisher};
use domain::value_objects::WrapUpId;
use uuid::Uuid;

use crate::wrapup::{commands::RequestWrapUpCommand, generate};

fn past_cmd() -> RequestWrapUpCommand {
    RequestWrapUpCommand {
        user_id: Some(Uuid::nil()),
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    }
}

#[tokio::test]
async fn creates_pending_record_and_emits_event() {
    let repo = InMemoryWrapUpRepository::new();
    let events = NoopEventPublisher::new();

    let id = generate::execute(repo.clone(), events.clone(), past_cmd())
        .await
        .unwrap();

    let stored = repo.store.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, id);
    assert_eq!(stored[0].status, WrapUpStatus::Pending);

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::WrapUpRequested { .. }))
    );
}

#[tokio::test]
async fn reuses_existing_ready_wrapup() {
    let repo = InMemoryWrapUpRepository::new();
    let existing_id = WrapUpId::generate();
    repo.store.lock().unwrap().push(WrapUpRecord {
        id: existing_id.clone(),
        user_id: Some(Uuid::nil()),
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        status: WrapUpStatus::Ready,
        report: None,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    });
    let events = NoopEventPublisher::new();

    let id = generate::execute(repo.clone(), events.clone(), past_cmd())
        .await
        .unwrap();
    assert_eq!(id, existing_id);
    assert_eq!(repo.store.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn replaces_failed_wrapup() {
    let repo = InMemoryWrapUpRepository::new();
    repo.store.lock().unwrap().push(WrapUpRecord {
        id: WrapUpId::generate(),
        user_id: Some(Uuid::nil()),
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        status: WrapUpStatus::Failed,
        report: None,
        error_message: Some("boom".into()),
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    });

    let events = NoopEventPublisher::new();

    let id = generate::execute(repo.clone(), events.clone(), past_cmd())
        .await
        .unwrap();

    let stored = repo.store.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, id);
    assert_eq!(stored[0].status, WrapUpStatus::Pending);
}

#[tokio::test]
async fn rejects_future_end_date() {
    let repo = InMemoryWrapUpRepository::new();
    let events = NoopEventPublisher::new();
    let err = generate::execute(
        repo.clone(),
        events.clone(),
        RequestWrapUpCommand {
            user_id: None,
            start_date: NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2031, 1, 1).unwrap(),
        },
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("future"));
}
