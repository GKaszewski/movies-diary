use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{PeriodicJob, RefreshSessionRepository},
};

pub struct RefreshSessionCleanupJob {
    refresh_session: Arc<dyn RefreshSessionRepository>,
}

impl RefreshSessionCleanupJob {
    pub fn new(refresh_session: Arc<dyn RefreshSessionRepository>) -> Self {
        Self { refresh_session }
    }
}

#[async_trait]
impl PeriodicJob for RefreshSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = self.refresh_session.delete_expired().await?;
        if n > 0 {
            tracing::info!("refresh session cleanup: removed {n} expired sessions");
        }
        Ok(())
    }
}
