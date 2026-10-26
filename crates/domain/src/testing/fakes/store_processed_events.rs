use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Timelike, Utc};

use crate::{
    errors::DomainError,
    events::EventId,
    ports::{Begin, ClaimToken, HANDLER_LEASE_SECONDS, ProcessedEventLedger},
};

use super::InMemoryStore;
use super::store::LedgerRow;

const STAMP: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

fn now_millis() -> NaiveDateTime {
    let now = Utc::now().naive_utc();
    now.with_nanosecond(now.nanosecond() / 1_000_000 * 1_000_000)
        .expect("truncated nanoseconds are valid")
}

fn token(at: NaiveDateTime) -> ClaimToken {
    ClaimToken::new(at.format(STAMP).to_string())
}

#[async_trait]
impl ProcessedEventLedger for InMemoryStore {
    async fn begin(&self, id: &EventId, handler: &str) -> Result<Begin, DomainError> {
        let now = now_millis();
        let lease = Duration::seconds(HANDLER_LEASE_SECONDS);
        let mut rows = self.processed_events.write().unwrap();
        let key = (id.value(), handler.to_owned());
        match rows.get_mut(&key) {
            None => {
                rows.insert(
                    key,
                    LedgerRow {
                        done: false,
                        started_at: now,
                        processed_at: now,
                    },
                );
                Ok(Begin::Run(token(now)))
            }
            Some(row) if row.done => Ok(Begin::AlreadyDone),
            Some(row) if row.started_at < now - lease => {
                row.started_at = now;
                row.processed_at = now;
                Ok(Begin::Run(token(now)))
            }
            Some(row) => {
                let remaining = (row.started_at + lease - now)
                    .to_std()
                    .unwrap_or_default()
                    .max(std::time::Duration::from_secs(1));
                Ok(Begin::InProgressElsewhere {
                    retry_after: remaining,
                })
            }
        }
    }

    async fn complete(
        &self,
        id: &EventId,
        handler: &str,
        owner: &ClaimToken,
    ) -> Result<(), DomainError> {
        if let Some(row) = self
            .processed_events
            .write()
            .unwrap()
            .get_mut(&(id.value(), handler.to_owned()))
            && token(row.started_at) == *owner
        {
            row.done = true;
            row.processed_at = Utc::now().naive_utc();
        }
        Ok(())
    }

    async fn release(
        &self,
        id: &EventId,
        handler: &str,
        owner: &ClaimToken,
    ) -> Result<(), DomainError> {
        let mut rows = self.processed_events.write().unwrap();
        let key = (id.value(), handler.to_owned());
        if rows
            .get(&key)
            .is_some_and(|r| !r.done && token(r.started_at) == *owner)
        {
            rows.remove(&key);
        }
        Ok(())
    }

    async fn delete_older_than(&self, cutoff: NaiveDateTime) -> Result<u64, DomainError> {
        let mut rows = self.processed_events.write().unwrap();
        let before = rows.len();
        rows.retain(|_, r| r.processed_at >= cutoff);
        Ok((before - rows.len()) as u64)
    }
}
