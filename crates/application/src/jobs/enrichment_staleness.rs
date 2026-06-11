use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, MovieProfileRepository, PeriodicJob},
    value_objects::ExternalMetadataId,
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
            let ext_id = match ExternalMetadataId::new(external_metadata_id) {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!("skipping stale movie with malformed external_metadata_id: {e}");
                    continue;
                }
            };
            let event = DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id: ext_id,
            };
            self.event_publisher.publish(&event).await?;
        }
        Ok(())
    }
}
