use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, MovieProfileRepository, PeriodicJob},
};

pub struct EnrichmentStalenessJob {
    movie_profile: Arc<dyn MovieProfileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl EnrichmentStalenessJob {
    pub fn new(
        movie_profile: Arc<dyn MovieProfileRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            movie_profile,
            event_publisher,
        }
    }
}

#[async_trait]
impl PeriodicJob for EnrichmentStalenessJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let stale = self.movie_profile.list_stale().await?;
        if stale.is_empty() {
            return Ok(());
        }
        tracing::info!("enrichment scan: {} stale movies", stale.len());
        for (movie_id, external_metadata_id) in stale {
            let event = DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            };
            self.event_publisher.publish(&event).await?;
        }
        Ok(())
    }
}
