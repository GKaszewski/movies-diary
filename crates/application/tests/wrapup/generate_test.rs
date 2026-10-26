use crate::support::request_wrap_up_command;
use chrono::NaiveDate;
use domain::events::DomainEvent;
use domain::models::wrapup::{WrapUpRecord, WrapUpStatus};
use domain::testing::{InMemoryWrapUpRepository, RefusingWrapUpWrites};
use domain::value_objects::WrapUpId;
use uuid::Uuid;

use std::sync::Arc;

use application::wrapup::deps::GenerateWrapUpDeps;
use application::wrapup::{commands::RequestWrapUpCommand, generate};

fn deps(repo: &Arc<InMemoryWrapUpRepository>) -> GenerateWrapUpDeps {
    GenerateWrapUpDeps {
        wrapup_repo: Arc::clone(repo) as _,
    }
}

fn past_cmd() -> RequestWrapUpCommand {
    request_wrap_up_command(Some(Uuid::nil()), 2024)
}

#[tokio::test]
async fn creates_pending_record_and_records_its_event_together() {
    let repo = InMemoryWrapUpRepository::new();

    let id = generate::execute(&deps(&repo), past_cmd()).await.unwrap();

    let stored = repo.store.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, id);
    assert_eq!(stored[0].status, WrapUpStatus::Pending);

    let recorded = repo.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        &recorded[0],
        DomainEvent::WrapUpRequested { wrapup_id, .. } if *wrapup_id == id
    ));
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

    let id = generate::execute(&deps(&repo), past_cmd()).await.unwrap();
    assert_eq!(id, existing_id);
    assert_eq!(repo.store.lock().unwrap().len(), 1);
    assert!(repo.recorded_events().is_empty());
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

    let id = generate::execute(&deps(&repo), past_cmd()).await.unwrap();

    let stored = repo.store.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, id);
    assert_eq!(stored[0].status, WrapUpStatus::Pending);
}

#[tokio::test]
async fn rejects_future_end_date() {
    let repo = InMemoryWrapUpRepository::new();
    let err = generate::execute(&deps(&repo), request_wrap_up_command(None, 2030))
        .await
        .unwrap_err();

    assert!(err.to_string().contains("future"));
}

#[tokio::test]
async fn a_refused_create_propagates_and_records_nothing() {
    let inner = InMemoryWrapUpRepository::new();
    let deps = GenerateWrapUpDeps {
        wrapup_repo: RefusingWrapUpWrites::new(Arc::clone(&inner)),
    };

    let result = generate::execute(&deps, past_cmd()).await;

    assert!(result.is_err());
    assert!(inner.store.lock().unwrap().is_empty());
    assert!(inner.recorded_events().is_empty());
}
