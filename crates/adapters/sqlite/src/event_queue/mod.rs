mod payload;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope, EventId},
    ports::{EventConsumer, EventPublisher},
};
use futures::stream::{self, BoxStream};
use sqlx::SqlitePool;
use tokio::sync::{Mutex, mpsc};

use crate::shared::enqueue_events;
pub(crate) use payload::DbEventPayload;

pub(crate) const CLAIM_LEASE_SECONDS: i64 = 600;

pub struct DbEventQueueConfig {
    pub poll_interval_ms: u64,
    pub batch_size: i64,
    pub max_attempts: i32,
}

impl DbEventQueueConfig {
    pub fn from_config(config: &config::EventsConfig) -> Self {
        Self {
            poll_interval_ms: config.queue_poll_interval_ms,
            batch_size: config.queue_batch_size,
            max_attempts: config.queue_max_attempts,
        }
    }
}

#[derive(Clone)]
pub struct SqliteEventQueue {
    pool: SqlitePool,
    config: Arc<DbEventQueueConfig>,
}

impl SqliteEventQueue {
    pub fn create(pool: SqlitePool, config: DbEventQueueConfig) -> Self {
        Self {
            pool,
            config: Arc::new(config),
        }
    }

    pub async fn create_publisher(
        pool: SqlitePool,
        config: &config::EventsConfig,
    ) -> anyhow::Result<Arc<dyn EventPublisher>> {
        let q = Self::create(pool, DbEventQueueConfig::from_config(config));
        Ok(Arc::new(q))
    }

    pub async fn create_channel(
        pool: SqlitePool,
        config: &config::EventsConfig,
    ) -> anyhow::Result<(Arc<dyn EventPublisher>, Arc<dyn EventConsumer>)> {
        let q = Self::create(pool, DbEventQueueConfig::from_config(config));
        Ok((Arc::new(q.clone()), Arc::new(q)))
    }
}

#[async_trait]
impl EventPublisher for SqliteEventQueue {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("begin: {e}")))?;
        enqueue_events(&mut tx, std::slice::from_ref(event)).await?;
        tx.commit()
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("commit: {e}")))?;

        Ok(())
    }
}

impl EventConsumer for SqliteEventQueue {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let pool = self.pool.clone();
        let config = Arc::clone(&self.config);
        let (tx, rx) = mpsc::channel(128);
        let rx = Arc::new(Mutex::new(rx));

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
                            let row_id = row.id;
                            let envelope = decode_row(&pool, row, config.max_attempts);
                            if let Err(e) = &envelope {
                                dead_letter_undecodable(&pool, row_id, e).await;
                            }
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
    id: i64,
    payload: String,
    event_id: Option<String>,
    attempts: i32,
    next_attempt_at: String,
    claimed_at: Option<String>,
}

async fn claim_batch(
    pool: &SqlitePool,
    config: &DbEventQueueConfig,
) -> Result<Vec<QueueRow>, DomainError> {
    let sql = format!(
        "UPDATE event_queue
         SET status = 'processing', claimed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id IN (
             SELECT id FROM event_queue
             WHERE (status = 'pending'
                    AND next_attempt_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                OR (status = 'processing'
                    AND (claimed_at IS NULL
                         OR claimed_at < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-{CLAIM_LEASE_SECONDS} seconds')))
             ORDER BY next_attempt_at ASC
             LIMIT ?
         )
         RETURNING id, payload, event_id, attempts, next_attempt_at, claimed_at"
    );
    let mut rows = sqlx::query_as::<_, QueueRow>(&sql)
        .bind(config.batch_size)
        .fetch_all(pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(format!("claim batch: {e}")))?;

    rows.sort_by(|a, b| (&a.next_attempt_at, a.id).cmp(&(&b.next_attempt_at, b.id)));
    Ok(rows)
}

async fn dead_letter_undecodable(pool: &SqlitePool, row_id: i64, error: &DomainError) {
    tracing::error!(
        row_id,
        "event queue row is undecodable, dead-lettering: {error}"
    );
    let result =
        sqlx::query("UPDATE event_queue SET status = 'dead_lettered', last_error = ? WHERE id = ?")
            .bind(format!("undecodable: {error}"))
            .bind(row_id)
            .execute(pool)
            .await;
    if let Err(e) = result {
        tracing::error!(row_id, "could not dead-letter undecodable row: {e}");
    }
}

fn decode_row(
    pool: &SqlitePool,
    row: QueueRow,
    max_attempts: i32,
) -> Result<EventEnvelope, DomainError> {
    let db_payload: DbEventPayload = serde_json::from_str(&row.payload)
        .map_err(|e| DomainError::InfrastructureError(format!("deserialize: {e}")))?;
    let event = DomainEvent::try_from(db_payload)?;
    let id = row
        .event_id
        .as_deref()
        .map(crate::shared::parse_uuid)
        .transpose()?
        .map(EventId::from_uuid);
    Ok(EventEnvelope::new(
        id,
        event,
        Box::new(DbAckHandle {
            pool: pool.clone(),
            row_id: row.id,
            attempts: row.attempts,
            max_attempts,
            claimed_at: row.claimed_at,
        }),
    ))
}

struct DbAckHandle {
    pool: SqlitePool,
    row_id: i64,
    attempts: i32,
    max_attempts: i32,
    claimed_at: Option<String>,
}

impl DbAckHandle {
    async fn settle<'q>(
        &self,
        what: &str,
        query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    ) -> Result<(), DomainError> {
        let done = query
            .bind(self.row_id)
            .bind(self.claimed_at.clone())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(format!("{what}: {e}")))?;
        if done.rows_affected() == 0 {
            tracing::warn!(
                row_id = self.row_id,
                "{what}: claim no longer held (lease lapsed and row reclaimed); ignoring"
            );
        }
        Ok(())
    }
}

#[async_trait]
impl AckHandle for DbAckHandle {
    async fn ack(&self) -> Result<(), DomainError> {
        self.settle(
            "ack",
            sqlx::query(
                "UPDATE event_queue SET status = 'done' \
                 WHERE id = ? AND status = 'processing' AND claimed_at IS ?",
            ),
        )
        .await
    }

    async fn nack(&self) -> Result<(), DomainError> {
        let new_attempts = self.attempts + 1;
        if new_attempts >= self.max_attempts {
            tracing::error!(
                row_id = self.row_id,
                attempts = new_attempts,
                "event exhausted max attempts, dead-lettering"
            );
            self.settle(
                "nack dead-letter",
                sqlx::query(
                    "UPDATE event_queue SET status = 'dead_lettered', attempts = ?, last_error = 'max attempts reached' \
                     WHERE id = ? AND status = 'processing' AND claimed_at IS ?",
                )
                .bind(new_attempts),
            )
            .await
        } else {
            let backoff = domain::events::retry_backoff(new_attempts.max(0) as u32).as_secs();
            let sql = format!(
                "UPDATE event_queue SET status = 'pending', attempts = ?, next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+{backoff} seconds'), last_error = 'nack' \
                 WHERE id = ? AND status = 'processing' AND claimed_at IS ?"
            );
            self.settle("nack retry", sqlx::query(&sql).bind(new_attempts))
                .await
        }
    }

    async fn defer(&self, delay: std::time::Duration) -> Result<(), DomainError> {
        let secs = delay.as_secs_f64().ceil().max(1.0) as i64;
        let sql = format!(
            "UPDATE event_queue
             SET status = 'pending', claimed_at = NULL, last_error = 'deferred',
                 next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+{secs} seconds')
             WHERE id = ? AND status = 'processing' AND claimed_at IS ?"
        );
        self.settle("defer", sqlx::query(&sql)).await
    }
}

#[cfg(test)]
mod tests;
