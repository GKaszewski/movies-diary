use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{AccountPurger, PeriodicJob},
};

pub const TOMBSTONE_RETENTION_DAYS: i64 = 7;

pub struct TombstoneCleanupJob {
    purger: Arc<dyn AccountPurger>,
}

impl TombstoneCleanupJob {
    pub fn new(purger: Arc<dyn AccountPurger>) -> Self {
        Self { purger }
    }
}

#[async_trait]
impl PeriodicJob for TombstoneCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(24 * 3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let cutoff =
            chrono::Utc::now().naive_utc() - chrono::Duration::days(TOMBSTONE_RETENTION_DAYS);
        let n = self.purger.purge_expired_tombstones(cutoff).await?;
        if n > 0 {
            tracing::info!("tombstone cleanup: removed {n} purged accounts");
        }
        Ok(())
    }
}
