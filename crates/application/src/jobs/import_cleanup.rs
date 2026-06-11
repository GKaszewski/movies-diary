use std::time::Duration;

use async_trait::async_trait;
use domain::{errors::DomainError, ports::PeriodicJob};

use crate::context::AppContext;

pub struct ImportSessionCleanupJob {
    ctx: AppContext,
}

impl ImportSessionCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for ImportSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::import::cleanup::execute(&self.ctx).await?;
        tracing::info!("import session cleanup: removed {} expired sessions", n);
        Ok(())
    }
}
