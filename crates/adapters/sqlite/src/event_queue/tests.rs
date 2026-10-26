use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use super::{DbEventQueueConfig, claim_batch};

async fn pool_on(path: &std::path::Path) -> SqlitePool {
    pool_with_timeout(path, 5).await
}

async fn pool_with_timeout(path: &std::path::Path, busy_secs: u64) -> SqlitePool {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(busy_secs));
    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .unwrap()
}

fn config(batch_size: i64) -> DbEventQueueConfig {
    DbEventQueueConfig {
        poll_interval_ms: 10,
        batch_size,
        max_attempts: 3,
    }
}

async fn insert(pool: &SqlitePool, n: usize) {
    for i in 0..n {
        sqlx::query("INSERT INTO event_queue (event_type, payload) VALUES ('t', ?)")
            .bind(format!("p{i}"))
            .execute(pool)
            .await
            .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claiming_while_another_connection_writes_does_not_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.db");
    let claimer = pool_on(&path).await;
    crate::migrate(&claimer).await.unwrap();
    let writer = pool_with_timeout(&path, 30).await;
    insert(&claimer, 50).await;

    let writer_task = {
        let writer = writer.clone();
        tokio::spawn(async move {
            for i in 0..2000 {
                sqlx::query("INSERT INTO event_queue (event_type, payload) VALUES ('t', ?)")
                    .bind(format!("w{i}"))
                    .execute(&writer)
                    .await
                    .unwrap();
            }
        })
    };

    let cfg = config(5);
    let mut claimed = 0;
    while !writer_task.is_finished() {
        let batch = claim_batch(&claimer, &cfg)
            .await
            .expect("claim must not fail while another connection writes");
        if batch.is_empty() {
            tokio::task::yield_now().await;
        }
        claimed += batch.len();
    }
    writer_task.await.unwrap();
    assert!(claimed > 0);
}

#[tokio::test]
async fn claimed_rows_are_marked_processing_and_not_claimed_twice() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool_on(&dir.path().join("q.db")).await;
    crate::migrate(&pool).await.unwrap();
    insert(&pool, 5).await;

    let cfg = config(3);
    let first = claim_batch(&pool, &cfg).await.unwrap();
    assert_eq!(first.len(), 3);
    let second = claim_batch(&pool, &cfg).await.unwrap();
    assert_eq!(second.len(), 2);
    assert!(claim_batch(&pool, &cfg).await.unwrap().is_empty());

    let mut ids: Vec<i64> = first.iter().chain(&second).map(|r| r.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5, "no row claimed twice");
    let (processing,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_queue WHERE status = 'processing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(processing, 5);
    let payloads: Vec<&str> = first.iter().map(|r| r.payload.as_str()).collect();
    assert_eq!(payloads, ["p0", "p1", "p2"], "oldest first");
}

mod identity {
    use domain::{
        events::DomainEvent,
        ports::{EventConsumer, EventPublisher},
        value_objects::UserId,
    };
    use futures::StreamExt;
    use sqlx::SqlitePool;

    use super::{DbEventQueueConfig, config};
    use crate::event_queue::SqliteEventQueue;

    async fn queue() -> (SqliteEventQueue, SqlitePool) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        let cfg: DbEventQueueConfig = config(10);
        (SqliteEventQueue::create(pool.clone(), cfg), pool)
    }

    fn event() -> DomainEvent {
        DomainEvent::UserUpdated {
            user_id: UserId::generate(),
        }
    }

    #[tokio::test]
    async fn published_events_get_a_unique_event_id() {
        let (q, pool) = queue().await;

        q.publish(&event()).await.unwrap();
        q.publish(&event()).await.unwrap();

        let ids: Vec<Option<String>> =
            sqlx::query_scalar("SELECT event_id FROM event_queue ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(ids.len(), 2);
        let a = ids[0].as_deref().expect("first event has an id");
        let b = ids[1].as_deref().expect("second event has an id");
        assert_ne!(a, b);
        uuid::Uuid::parse_str(a).expect("event id is a uuid");
    }

    #[tokio::test]
    async fn the_consumer_surfaces_the_published_event_id() {
        let (q, pool) = queue().await;
        q.publish(&event()).await.unwrap();
        let stored: String = sqlx::query_scalar("SELECT event_id FROM event_queue")
            .fetch_one(&pool)
            .await
            .unwrap();

        let envelope = q.consume().next().await.unwrap().unwrap();

        assert_eq!(envelope.id().unwrap().value().to_string(), stored);
    }

    #[tokio::test]
    async fn legacy_queue_rows_without_event_id_still_decode() {
        let (q, pool) = queue().await;
        let payload =
            serde_json::to_string(&crate::event_queue::DbEventPayload::from(&event())).unwrap();
        sqlx::query(
            "INSERT INTO event_queue (event_type, payload, event_id) VALUES ('UserUpdated', ?, NULL)",
        )
        .bind(payload)
        .execute(&pool)
        .await
        .unwrap();

        let envelope = q.consume().next().await.unwrap().unwrap();

        assert!(envelope.id().is_none());
    }

    #[tokio::test]
    async fn defer_reschedules_without_counting_an_attempt() {
        let (q, pool) = queue().await;
        q.publish(&event()).await.unwrap();
        let envelope = q.consume().next().await.unwrap().unwrap();

        envelope
            .defer(std::time::Duration::from_secs(600))
            .await
            .unwrap();

        let (status, attempts, claimed, due_in): (String, i32, Option<String>, i64) =
            sqlx::query_as(
                "SELECT status, attempts, claimed_at,
                        CAST(strftime('%s', next_attempt_at) AS INTEGER)
                          - CAST(strftime('%s', 'now') AS INTEGER)
                 FROM event_queue",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "pending");
        assert_eq!(attempts, 0, "a deferral is not a failed attempt");
        assert!(claimed.is_none(), "claim is released");
        assert!((595..=600).contains(&due_in), "due in {due_in}s");
    }
}

mod lease {
    use domain::ports::EventConsumer;
    use futures::StreamExt;

    use super::{claim_batch, config};
    use crate::event_queue::SqliteEventQueue;

    async fn pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        pool
    }

    async fn claim_then_age(pool: &sqlx::SqlitePool, claimed_at: Option<&str>) {
        super::insert(pool, 1).await;
        assert_eq!(claim_batch(pool, &config(10)).await.unwrap().len(), 1);
        sqlx::query("UPDATE event_queue SET claimed_at = ?")
            .bind(claimed_at)
            .execute(pool)
            .await
            .unwrap();
    }

    fn ago(seconds: i64) -> String {
        (chrono::Utc::now() - chrono::Duration::seconds(seconds))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    #[tokio::test]
    async fn an_expired_claim_is_reclaimed() {
        let pool = pool().await;
        claim_then_age(&pool, Some(&ago(super::super::CLAIM_LEASE_SECONDS + 60))).await;

        let again = claim_batch(&pool, &config(10)).await.unwrap();

        assert_eq!(again.len(), 1, "stranded row is claimed again");
        let claimed: String = sqlx::query_scalar("SELECT claimed_at FROM event_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(claimed > ago(60), "lease restarted: {claimed}");
    }

    #[tokio::test]
    async fn a_legacy_processing_row_without_claimed_at_counts_as_expired() {
        let pool = pool().await;
        claim_then_age(&pool, None).await;

        assert_eq!(claim_batch(&pool, &config(10)).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_fresh_claim_is_not_reclaimed() {
        let pool = pool().await;
        claim_then_age(&pool, Some(&ago(30))).await;

        assert!(claim_batch(&pool, &config(10)).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_undecodable_row_is_dead_lettered_not_left_processing() {
        let pool = pool().await;
        sqlx::query("INSERT INTO event_queue (event_type, payload) VALUES ('t', 'not json')")
            .execute(&pool)
            .await
            .unwrap();
        let q = SqliteEventQueue::create(pool.clone(), config(10));

        let first = q.consume().next().await.unwrap();
        assert!(first.is_err());

        let (status, error): (String, Option<String>) =
            sqlx::query_as("SELECT status, last_error FROM event_queue")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "dead_lettered");
        assert!(error.unwrap().contains("undecodable"));
    }
}

mod stale_handle {
    use domain::{
        events::{DomainEvent, EventEnvelope},
        ports::{EventConsumer, EventPublisher},
        value_objects::UserId,
    };
    use futures::StreamExt;

    use super::{claim_batch, config};
    use crate::event_queue::{CLAIM_LEASE_SECONDS, SqliteEventQueue};

    async fn reclaimed() -> (sqlx::SqlitePool, EventEnvelope) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        let q = SqliteEventQueue::create(pool.clone(), config(10));
        q.publish(&DomainEvent::UserUpdated {
            user_id: UserId::generate(),
        })
        .await
        .unwrap();
        let stale = q.consume().next().await.unwrap().unwrap();
        let old = (chrono::Utc::now() - chrono::Duration::seconds(CLAIM_LEASE_SECONDS + 60))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        sqlx::query("UPDATE event_queue SET claimed_at = ?")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(claim_batch(&pool, &config(10)).await.unwrap().len(), 1);
        let second_claim = (chrono::Utc::now() - chrono::Duration::seconds(5))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        sqlx::query("UPDATE event_queue SET claimed_at = ?")
            .bind(second_claim)
            .execute(&pool)
            .await
            .unwrap();
        (pool, stale)
    }

    async fn state(pool: &sqlx::SqlitePool) -> (String, i32) {
        sqlx::query_as("SELECT status, attempts FROM event_queue")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_stale_ack_cannot_complete_a_reclaimed_row() {
        let (pool, stale) = reclaimed().await;
        stale.ack().await.unwrap();
        assert_eq!(state(&pool).await, ("processing".into(), 0));
    }

    #[tokio::test]
    async fn a_stale_nack_cannot_spend_an_attempt_on_a_reclaimed_row() {
        let (pool, stale) = reclaimed().await;
        stale.nack().await.unwrap();
        assert_eq!(state(&pool).await, ("processing".into(), 0));
    }

    #[tokio::test]
    async fn a_stale_defer_cannot_release_a_reclaimed_row() {
        let (pool, stale) = reclaimed().await;
        stale
            .defer(std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(state(&pool).await, ("processing".into(), 0));
    }
}
