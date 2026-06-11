use std::time::Duration;

use async_trait::async_trait;
use domain::{errors::DomainError, ports::PeriodicJob};

use crate::context::AppContext;

pub struct RefreshSessionCleanupJob {
    ctx: AppContext,
}

impl RefreshSessionCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for RefreshSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = self.ctx.repos.refresh_session.delete_expired().await?;
        if n > 0 {
            tracing::info!("refresh session cleanup: removed {n} expired sessions");
        }
        Ok(())
    }
}
