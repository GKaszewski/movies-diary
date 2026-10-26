use application::jobs::{PROCESSED_EVENTS_RETENTION_DAYS, ProcessedEventsCleanupJob};
use domain::{events::EventId, ports::PeriodicJob, testing::InMemoryStore};

#[tokio::test]
async fn removes_only_rows_older_than_the_retention_window() {
    let store = InMemoryStore::new();
    let now = chrono::Utc::now().naive_utc();
    let stale = EventId::new();
    let fresh = EventId::new();
    store.seed_processed_event(
        &stale,
        "h",
        true,
        now - chrono::Duration::days(PROCESSED_EVENTS_RETENTION_DAYS + 1),
    );
    store.seed_processed_event(
        &fresh,
        "h",
        true,
        now - chrono::Duration::days(PROCESSED_EVENTS_RETENTION_DAYS - 1),
    );

    ProcessedEventsCleanupJob::new(store.clone())
        .run()
        .await
        .unwrap();

    use domain::ports::{Begin, ProcessedEventLedger};
    assert!(matches!(
        store.begin(&stale, "h").await.unwrap(),
        Begin::Run(_)
    ));
    assert_eq!(store.begin(&fresh, "h").await.unwrap(), Begin::AlreadyDone);
}

#[test]
fn runs_daily() {
    let job = ProcessedEventsCleanupJob::new(InMemoryStore::new());
    assert_eq!(job.interval(), std::time::Duration::from_secs(86_400));
}
