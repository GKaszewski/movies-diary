use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{PeriodicJob, WatchEventRepository},
};

pub struct WatchEventCleanupJob {
    watch_event: Arc<dyn WatchEventRepository>,
}

impl WatchEventCleanupJob {
    pub fn new(watch_event: Arc<dyn WatchEventRepository>) -> Self {
        Self { watch_event }
    }
}

#[async_trait]
impl PeriodicJob for WatchEventCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::integrations::cleanup::execute(self.watch_event.clone()).await?;
        if n > 0 {
            tracing::info!("watch event cleanup: removed {n} old entries");
        }
        Ok(())
    }
}
