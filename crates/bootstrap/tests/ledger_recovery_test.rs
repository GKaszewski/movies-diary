mod support;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::worker::WorkerService;
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope, EventId},
    ports::{Begin, EventConsumer, EventHandler, ProcessedEventLedger},
};
use futures::stream::BoxStream;
use sqlite::SqliteProcessedEventLedger;

struct Counting(Arc<AtomicU32>);

#[async_trait]
impl EventHandler for Counting {
    fn name(&self) -> &'static str {
        "crashy"
    }
    async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct Outcome(Arc<Mutex<Vec<&'static str>>>);

#[async_trait]
impl AckHandle for Outcome {
    async fn ack(&self) -> Result<(), DomainError> {
        self.0.lock().unwrap().push("ack");
        Ok(())
    }
    async fn nack(&self) -> Result<(), DomainError> {
        self.0.lock().unwrap().push("nack");
        Ok(())
    }
    async fn defer(&self, _: Duration) -> Result<(), DomainError> {
        self.0.lock().unwrap().push("defer");
        Ok(())
    }
}

struct Once(Mutex<Option<EventEnvelope>>);

impl EventConsumer for Once {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let item = self.0.lock().unwrap().take().map(Ok);
        Box::pin(futures::stream::iter(item))
    }
}

#[tokio::test]
async fn an_abandoned_claim_defers_deliveries_until_the_lease_lapses_then_runs_once() {
    let pool = support::memory_pool().await;
    sqlite::migrate(&pool).await.unwrap();
    let ledger = Arc::new(SqliteProcessedEventLedger::with_lease(
        pool,
        Duration::from_secs(1),
    ));
    let id = EventId::new();
    assert!(matches!(
        ledger.begin(&id, "crashy").await.unwrap(),
        Begin::Run(_)
    ));

    let runs = Arc::new(AtomicU32::new(0));
    let outcomes = Arc::new(Mutex::new(Vec::new()));
    let deliver = || {
        let envelope = EventEnvelope::new(
            Some(id),
            DomainEvent::SearchReindexRequested,
            Box::new(Outcome(outcomes.clone())),
        );
        let service = WorkerService::new(
            Arc::new(Once(Mutex::new(Some(envelope)))),
            vec![Arc::new(Counting(runs.clone()))],
            ledger.clone(),
        )
        .unwrap();
        async move { service.run(tokio::sync::watch::channel(false).1).await }
    };

    deliver().await;
    assert_eq!(*outcomes.lock().unwrap(), ["defer"]);
    assert_eq!(runs.load(Ordering::SeqCst), 0);

    tokio::time::sleep(Duration::from_millis(2100)).await;
    deliver().await;
    assert_eq!(*outcomes.lock().unwrap(), ["defer", "ack"]);
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    deliver().await;
    assert_eq!(runs.load(Ordering::SeqCst), 1, "done claims never re-run");
}
