use chrono::NaiveDate;
use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::wrapup::{DateRange, WrapUpRecord, WrapUpScope, WrapUpStatus};
use domain::ports::WrapUpRepository;
use domain::services::wrapup_analyzer;
use domain::testing::InMemoryWrapUpRepository;
use domain::value_objects::WrapUpId;

use application::wrapup::deps::GetReadyReportDeps;
use application::wrapup::get_ready_report;

fn year_range(year: i32) -> DateRange {
    DateRange::new(
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap(),
    )
    .unwrap()
}

fn record(
    user_id: Option<Uuid>,
    year: i32,
    status: WrapUpStatus,
    with_report: bool,
) -> WrapUpRecord {
    let range = year_range(year);
    let domain_scope = match user_id {
        Some(u) => WrapUpScope::User(u),
        None => WrapUpScope::Global,
    };
    let report = if with_report {
        Some(wrapup_analyzer::build_report(
            domain_scope,
            range.clone(),
            &[],
        ))
    } else {
        None
    };
    WrapUpRecord {
        id: WrapUpId::generate(),
        user_id,
        start_date: range.start(),
        end_date: range.end(),
        status,
        report,
        error_message: None,
        created_at: chrono::Utc::now().naive_utc(),
        completed_at: None,
    }
}

#[tokio::test]
async fn ready_report_returns_report_when_ready() {
    let repo = InMemoryWrapUpRepository::new();
    let user_id = Uuid::new_v4();
    repo.create(&record(Some(user_id), 2024, WrapUpStatus::Ready, true), &[])
        .await
        .unwrap();

    let deps = GetReadyReportDeps { wrapup_repo: repo };

    let report = get_ready_report::execute(&deps, WrapUpScope::User(user_id), 2024)
        .await
        .unwrap();
    assert_eq!(report.total_movies, 0);
}

#[tokio::test]
async fn ready_report_is_not_found_when_status_not_ready() {
    let repo = InMemoryWrapUpRepository::new();
    let user_id = Uuid::new_v4();
    repo.create(
        &record(Some(user_id), 2024, WrapUpStatus::Generating, false),
        &[],
    )
    .await
    .unwrap();

    let deps = GetReadyReportDeps { wrapup_repo: repo };

    let err = get_ready_report::execute(&deps, WrapUpScope::User(user_id), 2024)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn ready_report_is_not_found_when_report_absent() {
    let repo = InMemoryWrapUpRepository::new();
    let user_id = Uuid::new_v4();
    // Status is Ready but `report` is None — an inconsistent-but-representable
    // state the old handler guarded against explicitly (`match record.report {
    // Some(r) => ..., None => 404 }`), separate from the status gate above.
    repo.create(
        &record(Some(user_id), 2024, WrapUpStatus::Ready, false),
        &[],
    )
    .await
    .unwrap();

    let deps = GetReadyReportDeps { wrapup_repo: repo };

    let err = get_ready_report::execute(&deps, WrapUpScope::User(user_id), 2024)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn ready_report_is_not_found_when_no_record_exists() {
    let repo = InMemoryWrapUpRepository::new();
    let deps = GetReadyReportDeps { wrapup_repo: repo };

    let err = get_ready_report::execute(&deps, WrapUpScope::User(Uuid::new_v4()), 2024)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn ready_report_global_scope_ignores_user_id() {
    let repo = InMemoryWrapUpRepository::new();
    repo.create(&record(None, 2024, WrapUpStatus::Ready, true), &[])
        .await
        .unwrap();

    let deps = GetReadyReportDeps { wrapup_repo: repo };

    get_ready_report::execute(&deps, WrapUpScope::Global, 2024)
        .await
        .unwrap();
}

#[tokio::test]
async fn ready_report_is_validation_error_for_unrepresentable_year() {
    let repo = InMemoryWrapUpRepository::new();
    let deps = GetReadyReportDeps { wrapup_repo: repo };

    let err = get_ready_report::execute(&deps, WrapUpScope::Global, 999_999)
        .await
        .unwrap_err();
    assert!(
        matches!(err, DomainError::ValidationError(_)),
        "got {err:?}"
    );
}
