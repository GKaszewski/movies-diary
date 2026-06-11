use std::time::Duration;

use async_trait::async_trait;
use domain::{errors::DomainError, ports::PeriodicJob};

use crate::context::AppContext;

pub struct WatchEventCleanupJob {
    ctx: AppContext,
}

impl WatchEventCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for WatchEventCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::integrations::cleanup::execute(&self.ctx).await?;
        if n > 0 {
            tracing::info!("watch event cleanup: removed {n} old entries");
        }
        Ok(())
    }
}
