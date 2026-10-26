mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{DomainEvent, EventId},
    ports::{EventPublisher, EventRelaySink},
    value_objects::UserId,
};
use sqlite::SqliteEventQueue;
use uuid::Uuid;

#[derive(Default)]
struct FakeSink {
    fail: bool,
    fail_first: std::sync::atomic::AtomicUsize,
    slow: Option<Duration>,
    started: Notify,
    seen: Mutex<Vec<Option<EventId>>>,
}

#[async_trait]
impl EventRelaySink for FakeSink {
    async fn relay(&self, id: Option<&EventId>, _event: &DomainEvent) -> Result<(), DomainError> {
        self.seen.lock().unwrap().push(id.cloned());
        if let Some(delay) = self.slow {
            self.started.notify_one();
            tokio::time::sleep(delay).await;
        }
        let outage = self
            .fail_first
            .try_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |n| n.checked_sub(1),
            )
            .is_ok();
        if self.fail || outage {
            Err(DomainError::InfrastructureError("bus down".into()))
        } else {
            Ok(())
        }
    }
}

async fn outbox() -> (sqlx::SqlitePool, Arc<dyn EventPublisher>, SqliteEventQueue) {
    outbox_with_attempts(config::EventsConfig::default().queue_max_attempts).await
}

async fn outbox_with_attempts(
    max_attempts: i32,
) -> (sqlx::SqlitePool, Arc<dyn EventPublisher>, SqliteEventQueue) {
    let pool = support::memory_pool().await;
    sqlite::migrate(&pool).await.unwrap();
    let config = config::EventsConfig {
        queue_poll_interval_ms: 10,
        queue_max_attempts: max_attempts,
        ..Default::default()
    };
    let queue = SqliteEventQueue::create(
        pool.clone(),
        sqlite::DbEventQueueConfig::from_config(&config),
    );
    (pool.clone(), Arc::new(queue.clone()), queue)
}

async fn row(pool: &sqlx::SqlitePool) -> (String, i32, Option<String>) {
    sqlx::query_as("SELECT status, attempts, event_id FROM event_queue")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn wait_for(pool: &sqlx::SqlitePool, status: &str) {
    for _ in 0..800 {
        if row(pool).await.0 == status {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("row never reached {status}: {:?}", row(pool).await);
}

fn start(
    queue: SqliteEventQueue,
    sink: Arc<FakeSink>,
) -> (
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (stop, rx) = tokio::sync::watch::channel(false);
    let relay = bootstrap::outbox_relay::OutboxRelay::new(Arc::new(queue), sink)
        .with_backoff(|_| Duration::from_millis(10));
    (stop, tokio::spawn(relay.run(rx)))
}

fn event() -> DomainEvent {
    DomainEvent::UserUpdated {
        user_id: UserId::from_uuid(Uuid::new_v4()),
    }
}

#[tokio::test]
async fn relay_marks_rows_done_only_after_publish_succeeds() {
    let (pool, publisher, queue) = outbox().await;
    publisher.publish(&event()).await.unwrap();
    let sink = Arc::new(FakeSink::default());

    let (stop, relay) = start(queue, sink.clone());
    wait_for(&pool, "done").await;
    relay.abort();
    drop(stop);

    let (_, attempts, event_id) = row(&pool).await;
    assert_eq!(attempts, 0);
    let seen = sink.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].as_ref().map(|id| id.value().to_string()),
        event_id,
        "the sink gets the row's own event id"
    );
}

#[tokio::test]
async fn a_failing_bus_never_spends_the_rows_attempts_and_the_row_relays_on_recovery() {
    let (pool, publisher, queue) = outbox_with_attempts(2).await;
    publisher.publish(&event()).await.unwrap();
    let sink = Arc::new(FakeSink {
        fail_first: std::sync::atomic::AtomicUsize::new(3),
        ..Default::default()
    });

    let (stop, relay) = start(queue, sink.clone());
    wait_for(&pool, "done").await;
    relay.abort();
    drop(stop);

    let (status, attempts, _) = row(&pool).await;
    assert_eq!((status.as_str(), attempts), ("done", 0));
    assert_eq!(
        sink.seen.lock().unwrap().len(),
        4,
        "3 failures then success"
    );
}

#[tokio::test]
async fn a_failed_publish_defers_the_row_without_counting_an_attempt() {
    let (pool, publisher, queue) = outbox().await;
    publisher.publish(&event()).await.unwrap();
    let sink = Arc::new(FakeSink {
        fail: true,
        ..Default::default()
    });

    let (stop, relay) = start(queue, sink.clone());
    for _ in 0..200 {
        let last_error: Option<String> = sqlx::query_scalar("SELECT last_error FROM event_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        if last_error.as_deref() == Some("deferred") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    relay.abort();
    drop(stop);

    let (status, attempts, _) = row(&pool).await;
    assert_eq!((status.as_str(), attempts), ("pending", 0));
    let last_error: Option<String> = sqlx::query_scalar("SELECT last_error FROM event_queue")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(last_error.as_deref(), Some("deferred"));
}

#[tokio::test]
async fn shutdown_lets_the_in_flight_row_finish_then_stops_the_relay() {
    let (pool, publisher, queue) = outbox().await;
    publisher.publish(&event()).await.unwrap();
    publisher.publish(&event()).await.unwrap();
    let sink = Arc::new(FakeSink {
        slow: Some(Duration::from_millis(200)),
        ..Default::default()
    });
    let (stop, relay) = start(queue, sink.clone());

    sink.started.notified().await;
    stop.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(5), relay)
        .await
        .expect("relay stops on shutdown")
        .unwrap();

    let statuses: Vec<String> = sqlx::query_scalar("SELECT status FROM event_queue ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(statuses[0], "done", "in-flight row was completed");
    assert_ne!(statuses[0], "processing");
    assert_eq!(sink.seen.lock().unwrap().len(), 1, "no new row after stop");
}
