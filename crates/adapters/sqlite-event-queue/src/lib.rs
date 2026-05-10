mod migrations;
mod payload;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope},
    ports::{EventConsumer, EventPublisher},
};
use futures::stream::{self, BoxStream};
use sqlx::SqlitePool;
use tokio::sync::{Mutex, mpsc};

use payload::DbEventPayload;

pub struct DbEventQueueConfig {
    pub poll_interval_ms: u64,
    pub batch_size:       i64,
    pub max_attempts:     i32,
}

impl DbEventQueueConfig {
    pub fn from_env() -> Self {
        Self {
            poll_interval_ms: std::env::var("EVENT_QUEUE_POLL_INTERVAL_MS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(500),
            batch_size: std::env::var("EVENT_QUEUE_BATCH_SIZE")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(10),
            max_attempts: std::env::var("EVENT_QUEUE_MAX_ATTEMPTS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(5),
        }
    }
}

#[derive(Clone)]
pub struct SqliteEventQueue {
    pool:   SqlitePool,
    config: Arc<DbEventQueueConfig>,
}

impl SqliteEventQueue {
    pub async fn create(pool: SqlitePool, config: DbEventQueueConfig) -> anyhow::Result<Self> {
        migrations::run(&pool).await?;
        Ok(Self { pool, config: Arc::new(config) })
    }

    pub async fn create_publisher(pool: SqlitePool) -> anyhow::Result<Arc<dyn EventPublisher>> {
        let q = Self::create(pool, DbEventQueueConfig::from_env()).await?;
        Ok(Arc::new(q))
    }

    pub async fn create_channel(
        pool: SqlitePool,
    ) -> anyhow::Result<(Arc<dyn EventPublisher>, Arc<dyn EventConsumer>)> {
        let q = Self::create(pool, DbEventQueueConfig::from_env()).await?;
        Ok((Arc::new(q.clone()), Arc::new(q)))
    }
}

#[async_trait]
impl EventPublisher for SqliteEventQueue {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let db_payload = DbEventPayload::from(event);
        let event_type = db_payload.event_type();
        let payload_json = serde_json::to_string(&db_payload)
            .map_err(|e| DomainError::InfrastructureError(format!("serialize: {e}")))?;

        sqlx::query(
            "INSERT INTO event_queue (event_type, payload) VALUES (?, ?)"
        )
        .bind(event_type)
        .bind(payload_json)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(format!("insert event: {e}")))?;

        Ok(())
    }
}

impl EventConsumer for SqliteEventQueue {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let pool           = self.pool.clone();
        let config         = Arc::clone(&self.config);
        let (tx, rx)       = mpsc::channel(128);
        let rx             = Arc::new(Mutex::new(rx));

        tokio::spawn(async move {
            let poll_interval = Duration::from_millis(config.poll_interval_ms);
            loop {
                match claim_batch(&pool, &config).await {
                    Err(e) => {
                        tracing::error!("sqlite event queue claim error: {e}");
                        tokio::time::sleep(poll_interval).await;
                    }
                    Ok(rows) if rows.is_empty() => {
                        tokio::time::sleep(poll_interval).await;
                    }
                    Ok(rows) => {
                        for row in rows {
                            let envelope = decode_row(&pool, row, config.max_attempts);
                            if tx.send(envelope).await.is_err() {
                                tracing::info!("sqlite event queue consumer closed");
                                return;
                            }
                        }
                        // no sleep — re-poll immediately when batch was non-empty
                    }
                }
            }
        });

        Box::pin(stream::unfold(rx, |rx| async move {
            let item = rx.lock().await.recv().await?;
            Some((item, rx))
        }))
    }
}

// ── Internal types ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct QueueRow {
    id:       i64,
    payload:  String,
    attempts: i32,
}

async fn claim_batch(
    pool:   &SqlitePool,
    config: &DbEventQueueConfig,
) -> Result<Vec<QueueRow>, DomainError> {
    let mut tx = pool.begin().await
        .map_err(|e| DomainError::InfrastructureError(format!("begin tx: {e}")))?;

    let rows = sqlx::query_as::<_, QueueRow>(
        "SELECT id, payload, attempts FROM event_queue
         WHERE status = 'pending'
           AND next_attempt_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         ORDER BY next_attempt_at ASC
         LIMIT ?"
    )
    .bind(config.batch_size)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| DomainError::InfrastructureError(format!("select pending: {e}")))?;

    if rows.is_empty() {
        tx.rollback().await.ok();
        return Ok(vec![]);
    }

    let placeholders = rows.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "UPDATE event_queue SET status = 'processing' WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for r in &rows { q = q.bind(r.id); }
    q.execute(&mut *tx).await
        .map_err(|e| DomainError::InfrastructureError(format!("mark processing: {e}")))?;

    tx.commit().await
        .map_err(|e| DomainError::InfrastructureError(format!("commit claim: {e}")))?;

    Ok(rows)
}

fn decode_row(
    pool:         &SqlitePool,
    row:          QueueRow,
    max_attempts: i32,
) -> Result<EventEnvelope, DomainError> {
    let db_payload: DbEventPayload = serde_json::from_str(&row.payload)
        .map_err(|e| DomainError::InfrastructureError(format!("deserialize: {e}")))?;
    let event = DomainEvent::try_from(db_payload)?;
    Ok(EventEnvelope::new(event, Box::new(DbAckHandle {
        pool:         pool.clone(),
        row_id:       row.id,
        attempts:     row.attempts,
        max_attempts,
    })))
}

struct DbAckHandle {
    pool:         SqlitePool,
    row_id:       i64,
    attempts:     i32,
    max_attempts: i32,
}

#[async_trait]
impl AckHandle for DbAckHandle {
    async fn ack(&self) -> Result<(), DomainError> {
        sqlx::query("UPDATE event_queue SET status = 'done' WHERE id = ?")
            .bind(self.row_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("ack: {e}")))?;
        Ok(())
    }

    async fn nack(&self) -> Result<(), DomainError> {
        let new_attempts = self.attempts + 1;
        if new_attempts >= self.max_attempts {
            sqlx::query(
                "UPDATE event_queue SET status = 'dead_lettered', attempts = ?, last_error = 'max attempts reached' WHERE id = ?"
            )
            .bind(new_attempts)
            .bind(self.row_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("nack dead-letter: {e}")))?;
        } else {
            let backoff = backoff_seconds(new_attempts);
            let sql = format!(
                "UPDATE event_queue SET status = 'pending', attempts = ?, next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+{backoff} seconds'), last_error = 'nack' WHERE id = ?"
            );
            sqlx::query(&sql)
                .bind(new_attempts)
                .bind(self.row_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(format!("nack retry: {e}")))?;
        }
        Ok(())
    }
}

fn backoff_seconds(attempts: i32) -> i64 {
    let base: i64 = 5 * (1i64 << attempts.min(6));
    base.min(300)
}
