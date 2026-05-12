use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, ImageRefQuery, PeriodicJob},
};

pub struct ConversionBackfillJob {
    image_ref: Arc<dyn ImageRefQuery>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl ConversionBackfillJob {
    pub fn new(
        image_ref: Arc<dyn ImageRefQuery>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self { image_ref, event_publisher }
    }
}

#[async_trait]
impl PeriodicJob for ConversionBackfillJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(60 * 60 * 24) // 24h
    }

    async fn run(&self) -> Result<(), DomainError> {
        let keys = self.image_ref.list_keys().await?;

        for key in keys {
            if key.ends_with(".avif") || key.ends_with(".webp") {
                continue;
            }
            if let Err(e) = self.event_publisher
                .publish(&DomainEvent::ImageStored { key: key.clone() })
                .await
            {
                tracing::warn!("backfill: failed to emit ImageStored for {key}: {e}");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/backfill.rs"]
mod tests;
