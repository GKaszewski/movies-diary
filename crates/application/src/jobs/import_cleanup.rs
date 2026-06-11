use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{errors::DomainError, ports::{ImportSessionRepository, PeriodicJob}};

pub struct ImportSessionCleanupJob {
    import_session: Arc<dyn ImportSessionRepository>,
}

impl ImportSessionCleanupJob {
    pub fn new(import_session: Arc<dyn ImportSessionRepository>) -> Self {
        Self { import_session }
    }
}

#[async_trait]
impl PeriodicJob for ImportSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::import::cleanup::execute(self.import_session.clone()).await?;
        tracing::info!("import session cleanup: removed {} expired sessions", n);
        Ok(())
    }
}
