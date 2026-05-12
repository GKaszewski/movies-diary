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
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockImageRef {
        keys: Vec<String>,
    }

    #[async_trait::async_trait]
    impl ImageRefQuery for MockImageRef {
        async fn list_keys(&self) -> Result<Vec<String>, DomainError> {
            Ok(self.keys.clone())
        }
    }

    struct MockPublisher {
        emitted: Mutex<Vec<String>>,
    }

    impl MockPublisher {
        fn new() -> Arc<Self> {
            Arc::new(Self { emitted: Mutex::new(vec![]) })
        }

        fn emitted(&self) -> Vec<String> {
            self.emitted.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockPublisher {
        async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
            if let DomainEvent::ImageStored { key } = event {
                self.emitted.lock().unwrap().push(key.clone());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn emits_image_stored_for_unconverted_keys() {
        let image_ref = Arc::new(MockImageRef {
            keys: vec!["avatars/u1".into(), "posters/m1".into()],
        });
        let publisher = MockPublisher::new();
        let job = ConversionBackfillJob::new(
            image_ref,
            Arc::clone(&publisher) as Arc<dyn EventPublisher>,
        );

        job.run().await.unwrap();

        let mut emitted = publisher.emitted();
        emitted.sort();
        assert_eq!(emitted, vec!["avatars/u1", "posters/m1"]);
    }

    #[tokio::test]
    async fn skips_already_converted_keys() {
        let image_ref = Arc::new(MockImageRef {
            keys: vec![
                "avatars/u1.avif".into(),
                "posters/m1".into(),
                "avatars/u2.webp".into(),
            ],
        });
        let publisher = MockPublisher::new();
        let job = ConversionBackfillJob::new(
            image_ref,
            Arc::clone(&publisher) as Arc<dyn EventPublisher>,
        );

        job.run().await.unwrap();

        assert_eq!(publisher.emitted(), vec!["posters/m1"]);
    }

    #[tokio::test]
    async fn empty_keys_emits_nothing() {
        let image_ref = Arc::new(MockImageRef { keys: vec![] });
        let publisher = MockPublisher::new();
        let job = ConversionBackfillJob::new(
            image_ref,
            Arc::clone(&publisher) as Arc<dyn EventPublisher>,
        );

        job.run().await.unwrap();

        assert!(publisher.emitted().is_empty());
    }
}
