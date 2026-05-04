use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventPublisher};
use tokio::sync::mpsc;

pub struct EventPublisherConfig {
    pub channel_buffer: usize,
}

impl EventPublisherConfig {
    pub fn from_env() -> Self {
        let channel_buffer = std::env::var("EVENT_CHANNEL_BUFFER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(128);
        Self { channel_buffer }
    }
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

pub struct ChannelEventPublisher {
    sender: mpsc::Sender<DomainEvent>,
}

#[async_trait]
impl EventPublisher for ChannelEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        self.sender
            .send(event.clone())
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

pub struct EventWorker {
    receiver: mpsc::Receiver<DomainEvent>,
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventWorker {
    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match &event {
                DomainEvent::ReviewLogged {
                    review_id,
                    movie_id,
                    user_id,
                    rating,
                    watched_at,
                } => {
                    tracing::info!(
                        review_id  = %review_id.value(),
                        movie_id   = %movie_id.value(),
                        user_id    = %user_id.value(),
                        rating     = rating.value(),
                        watched_at = %watched_at,
                        "event: review_logged"
                    );
                }
                DomainEvent::MovieDiscovered {
                    movie_id,
                    external_metadata_id,
                } => {
                    tracing::info!(
                        movie_id    = %movie_id.value(),
                        external_id = external_metadata_id.value(),
                        "event: movie_discovered"
                    );
                }
            }
            for handler in &self.handlers {
                if let Err(e) = handler.handle(&event).await {
                    tracing::error!("event handler error: {e}");
                }
            }
        }
        tracing::info!("event worker shut down");
    }
}

pub fn create_event_channel(
    config: EventPublisherConfig,
    handlers: Vec<Box<dyn EventHandler>>,
) -> (ChannelEventPublisher, EventWorker) {
    let (tx, rx) = mpsc::channel(config.channel_buffer);
    (
        ChannelEventPublisher { sender: tx },
        EventWorker {
            receiver: rx,
            handlers,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;
    use domain::{
        errors::DomainError,
        events::DomainEvent,
        value_objects::{ExternalMetadataId, MovieId},
    };

    struct RecordingHandler {
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl EventHandler for RecordingHandler {
        async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
            let label = match event {
                DomainEvent::MovieDiscovered { .. } => "movie_discovered",
                DomainEvent::ReviewLogged { .. } => "review_logged",
            };
            self.calls.lock().unwrap().push(label.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn single_handler_receives_event() {
        let calls = Arc::new(Mutex::new(vec![]));
        let handler = RecordingHandler { calls: Arc::clone(&calls) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(config, vec![Box::new(handler)]);

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(*calls.lock().unwrap(), vec!["movie_discovered"]);
    }

    #[tokio::test]
    async fn multiple_handlers_all_receive_event() {
        let calls1 = Arc::new(Mutex::new(vec![]));
        let calls2 = Arc::new(Mutex::new(vec![]));
        let handler1 = RecordingHandler { calls: Arc::clone(&calls1) };
        let handler2 = RecordingHandler { calls: Arc::clone(&calls2) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(
            config,
            vec![Box::new(handler1), Box::new(handler2)],
        );

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt9999999".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(calls1.lock().unwrap().len(), 1);
        assert_eq!(calls2.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn handler_error_does_not_stop_worker() {
        struct FailingHandler;
        #[async_trait]
        impl EventHandler for FailingHandler {
            async fn handle(&self, _: &DomainEvent) -> Result<(), DomainError> {
                Err(DomainError::InfrastructureError("boom".into()))
            }
        }

        let calls = Arc::new(Mutex::new(vec![]));
        let good = RecordingHandler { calls: Arc::clone(&calls) };
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, worker) = create_event_channel(
            config,
            vec![Box::new(FailingHandler), Box::new(good)],
        );

        tokio::spawn(worker.run());

        let event = DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt0000001".into()).unwrap(),
        };
        publisher.publish(&event).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(calls.lock().unwrap().len(), 1);
    }
}
