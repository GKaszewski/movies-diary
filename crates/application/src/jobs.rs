use std::time::Duration;

use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::PeriodicJob};

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
        let n = crate::use_cases::cleanup_expired_import_sessions::execute(&self.ctx).await?;
        tracing::info!("import session cleanup: removed {} expired sessions", n);
        Ok(())
    }
}

pub struct EnrichmentStalenessJob {
    ctx: AppContext,
}

impl EnrichmentStalenessJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for EnrichmentStalenessJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let stale = self.ctx.movie_profile_repository.list_stale().await?;
        if stale.is_empty() {
            return Ok(());
        }
        tracing::info!("enrichment scan: {} stale movies", stale.len());
        for (movie_id, external_metadata_id) in stale {
            let event = DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            };
            self.ctx.event_publisher.publish(&event).await?;
        }
        Ok(())
    }
}
