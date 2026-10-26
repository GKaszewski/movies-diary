use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{PeriodicJob, ProcessedEventLedger},
};

pub const PROCESSED_EVENTS_RETENTION_DAYS: i64 = 30;

pub struct ProcessedEventsCleanupJob {
    ledger: Arc<dyn ProcessedEventLedger>,
}

impl ProcessedEventsCleanupJob {
    pub fn new(ledger: Arc<dyn ProcessedEventLedger>) -> Self {
        Self { ledger }
    }
}

#[async_trait]
impl PeriodicJob for ProcessedEventsCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(24 * 3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let cutoff = chrono::Utc::now().naive_utc()
            - chrono::Duration::days(PROCESSED_EVENTS_RETENTION_DAYS);
        let n = self.ledger.delete_older_than(cutoff).await?;
        if n > 0 {
            tracing::info!("processed events cleanup: removed {n} ledger rows");
        }
        Ok(())
    }
}
