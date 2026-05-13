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
        Arc::new(Self {
            emitted: Mutex::new(vec![]),
        })
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
    let job =
        ConversionBackfillJob::new(image_ref, Arc::clone(&publisher) as Arc<dyn EventPublisher>);

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
    let job =
        ConversionBackfillJob::new(image_ref, Arc::clone(&publisher) as Arc<dyn EventPublisher>);

    job.run().await.unwrap();

    assert_eq!(publisher.emitted(), vec!["posters/m1"]);
}

#[tokio::test]
async fn empty_keys_emits_nothing() {
    let image_ref = Arc::new(MockImageRef { keys: vec![] });
    let publisher = MockPublisher::new();
    let job =
        ConversionBackfillJob::new(image_ref, Arc::clone(&publisher) as Arc<dyn EventPublisher>);

    job.run().await.unwrap();

    assert!(publisher.emitted().is_empty());
}
