use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::EventId,
    ports::{Begin, ClaimToken, HANDLER_LEASE_SECONDS, ProcessedEventLedger},
};
use sqlx::SqlitePool;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";
const CLAIM_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

pub struct SqliteProcessedEventLedger {
    pool: SqlitePool,
    lease: chrono::Duration,
}

impl SqliteProcessedEventLedger {
    pub fn new(pool: SqlitePool) -> Self {
        Self::with_lease(
            pool,
            std::time::Duration::from_secs(HANDLER_LEASE_SECONDS as u64),
        )
    }

    pub fn with_lease(pool: SqlitePool, lease: std::time::Duration) -> Self {
        Self {
            pool,
            lease: chrono::Duration::from_std(lease).expect("lease fits a chrono duration"),
        }
    }
}

fn stamp(at: chrono::NaiveDateTime) -> String {
    at.format(TIMESTAMP_FORMAT).to_string()
}

fn claim_stamp(at: chrono::NaiveDateTime) -> String {
    at.format(CLAIM_FORMAT).to_string()
}

fn parse_claim_stamp(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, CLAIM_FORMAT)
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, TIMESTAMP_FORMAT))
        .ok()
}

#[async_trait]
impl ProcessedEventLedger for SqliteProcessedEventLedger {
    async fn begin(&self, id: &EventId, handler: &str) -> Result<Begin, DomainError> {
        for _ in 0..2 {
            let now = chrono::Utc::now().naive_utc();
            let started = claim_stamp(now);
            let claimed = sqlx::query(
                "INSERT INTO processed_events (event_id, handler, status, started_at, processed_at)
                 VALUES (?, ?, 'in_progress', ?, ?)
                 ON CONFLICT (event_id, handler) DO UPDATE
                   SET started_at = excluded.started_at, processed_at = excluded.processed_at
                 WHERE processed_events.status = 'in_progress'
                   AND processed_events.started_at < ?",
            )
            .bind(id.value().to_string())
            .bind(handler)
            .bind(&started)
            .bind(stamp(now))
            .bind(claim_stamp(now - self.lease))
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?
            .rows_affected();
            if claimed == 1 {
                return Ok(Begin::Run(ClaimToken::new(started)));
            }

            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT status, started_at FROM processed_events WHERE event_id = ? AND handler = ?",
            )
            .bind(id.value().to_string())
            .bind(handler)
            .fetch_optional(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
            match row {
                Some((status, _)) if status == "done" => return Ok(Begin::AlreadyDone),
                Some((_, started_at)) => {
                    let lease_end = started_at
                        .as_deref()
                        .and_then(parse_claim_stamp)
                        .map_or(now + self.lease, |s| s + self.lease);
                    let retry_after = (lease_end - now)
                        .to_std()
                        .unwrap_or_default()
                        .max(std::time::Duration::from_secs(1));
                    return Ok(Begin::InProgressElsewhere { retry_after });
                }
                None => continue,
            }
        }
        Ok(Begin::InProgressElsewhere {
            retry_after: std::time::Duration::from_secs(1),
        })
    }

    async fn complete(
        &self,
        id: &EventId,
        handler: &str,
        token: &ClaimToken,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE processed_events SET status = 'done', processed_at = ?
             WHERE event_id = ? AND handler = ? AND status = 'in_progress' AND started_at = ?",
        )
        .bind(stamp(chrono::Utc::now().naive_utc()))
        .bind(id.value().to_string())
        .bind(handler)
        .bind(token.as_str())
        .execute(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        Ok(())
    }

    async fn release(
        &self,
        id: &EventId,
        handler: &str,
        token: &ClaimToken,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "DELETE FROM processed_events
             WHERE event_id = ? AND handler = ? AND status = 'in_progress' AND started_at = ?",
        )
        .bind(id.value().to_string())
        .bind(handler)
        .bind(token.as_str())
        .execute(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        Ok(())
    }

    async fn delete_older_than(&self, cutoff: chrono::NaiveDateTime) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM processed_events WHERE processed_at < ?")
            .bind(stamp(cutoff))
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        pool
    }

    async fn ledger() -> (SqliteProcessedEventLedger, SqlitePool) {
        let pool = memory_pool().await;
        (SqliteProcessedEventLedger::new(pool.clone()), pool)
    }

    async fn insert(pool: &SqlitePool, id: &EventId, status: &str, at: &str) {
        sqlx::query(
            "INSERT INTO processed_events (event_id, handler, status, started_at, processed_at)
             VALUES (?, 'h', ?, ?, ?)",
        )
        .bind(id.value().to_string())
        .bind(status)
        .bind(at)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
    }

    fn token(begin: Begin) -> ClaimToken {
        match begin {
            Begin::Run(token) => token,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    fn blocked(begin: Begin) -> std::time::Duration {
        match begin {
            Begin::InProgressElsewhere { retry_after } => retry_after,
            other => panic!("expected InProgressElsewhere, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_first_begin_runs_and_a_second_sees_it_in_progress() {
        let (ledger, _) = ledger().await;
        let id = EventId::new();

        token(ledger.begin(&id, "a").await.unwrap());
        let retry_after = blocked(ledger.begin(&id, "a").await.unwrap());
        token(ledger.begin(&id, "b").await.unwrap());
        token(ledger.begin(&EventId::new(), "a").await.unwrap());

        let lease = std::time::Duration::from_secs(HANDLER_LEASE_SECONDS as u64);
        assert!(retry_after <= lease && retry_after > lease - std::time::Duration::from_secs(5));
    }

    #[tokio::test]
    async fn a_completed_claim_is_already_done() {
        let (ledger, _) = ledger().await;
        let id = EventId::new();
        let t = token(ledger.begin(&id, "a").await.unwrap());

        ledger.complete(&id, "a", &t).await.unwrap();

        assert_eq!(ledger.begin(&id, "a").await.unwrap(), Begin::AlreadyDone);
    }

    #[tokio::test]
    async fn a_released_claim_can_run_again() {
        let (ledger, _) = ledger().await;
        let id = EventId::new();
        let t = token(ledger.begin(&id, "a").await.unwrap());

        ledger.release(&id, "a", &t).await.unwrap();

        token(ledger.begin(&id, "a").await.unwrap());
    }

    #[tokio::test]
    async fn release_never_removes_a_done_claim() {
        let (ledger, _) = ledger().await;
        let id = EventId::new();
        let t = token(ledger.begin(&id, "a").await.unwrap());
        ledger.complete(&id, "a", &t).await.unwrap();

        ledger.release(&id, "a", &t).await.unwrap();

        assert_eq!(ledger.begin(&id, "a").await.unwrap(), Begin::AlreadyDone);
    }

    #[tokio::test]
    async fn a_stale_in_progress_claim_is_taken_over() {
        let (ledger, pool) = ledger().await;
        let id = EventId::new();
        insert(&pool, &id, "in_progress", "2020-01-01T00:00:00Z").await;

        token(ledger.begin(&id, "h").await.unwrap());
        blocked(ledger.begin(&id, "h").await.unwrap());
        let processed_at: String = sqlx::query_scalar("SELECT processed_at FROM processed_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            processed_at.as_str() > "2020-01-01T00:00:00Z",
            "takeover refreshes processed_at"
        );
    }

    #[tokio::test]
    async fn a_done_claim_is_never_taken_over_however_old() {
        let (ledger, pool) = ledger().await;
        let id = EventId::new();
        insert(&pool, &id, "done", "2020-01-01T00:00:00Z").await;

        assert_eq!(ledger.begin(&id, "h").await.unwrap(), Begin::AlreadyDone);
    }

    #[tokio::test]
    async fn a_lapsed_owner_cannot_release_a_claim_taken_over_by_another() {
        let pool = memory_pool().await;
        let ledger =
            SqliteProcessedEventLedger::with_lease(pool.clone(), std::time::Duration::from_secs(1));
        let id = EventId::new();
        let lapsed = token(ledger.begin(&id, "a").await.unwrap());
        tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
        let owner = token(ledger.begin(&id, "a").await.unwrap());
        assert_ne!(lapsed, owner);

        ledger.release(&id, "a", &lapsed).await.unwrap();
        ledger.complete(&id, "a", &lapsed).await.unwrap();

        blocked(ledger.begin(&id, "a").await.unwrap());
        ledger.complete(&id, "a", &owner).await.unwrap();
        assert_eq!(ledger.begin(&id, "a").await.unwrap(), Begin::AlreadyDone);
    }

    #[tokio::test]
    async fn concurrent_begins_have_exactly_one_winner() {
        let dir = tempfile::tempdir().unwrap();
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("l.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        let ledger = std::sync::Arc::new(SqliteProcessedEventLedger::new(pool));
        let id = EventId::new();

        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let ledger = ledger.clone();
                tokio::spawn(async move { ledger.begin(&id, "a").await.unwrap() })
            })
            .collect();
        let mut runs = 0;
        for t in tasks {
            if matches!(t.await.unwrap(), Begin::Run(_)) {
                runs += 1;
            }
        }

        assert_eq!(runs, 1);
    }

    #[tokio::test]
    async fn delete_older_than_removes_only_rows_before_the_cutoff() {
        let (ledger, pool) = ledger().await;
        let (old_done, old_stuck, recent) = (EventId::new(), EventId::new(), EventId::new());
        insert(&pool, &old_done, "done", "2026-01-01T00:00:00Z").await;
        insert(&pool, &old_stuck, "in_progress", "2026-01-02T00:00:00Z").await;
        insert(&pool, &recent, "done", "2026-06-01T00:00:00Z").await;
        let cutoff = chrono::NaiveDate::from_ymd_opt(2026, 3, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let removed = ledger.delete_older_than(cutoff).await.unwrap();

        assert_eq!(removed, 2);
        assert_eq!(
            ledger.begin(&recent, "h").await.unwrap(),
            Begin::AlreadyDone
        );
    }
}
