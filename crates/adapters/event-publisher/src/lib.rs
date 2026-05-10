use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope},
    ports::{EventConsumer, EventPublisher},
};
use futures::stream::{self, BoxStream};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub use domain::ports::EventHandler;

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

struct NoopAck;

#[async_trait]
impl AckHandle for NoopAck {
    async fn ack(&self) -> Result<(), DomainError> { Ok(()) }
    async fn nack(&self) -> Result<(), DomainError> { Ok(()) }
}

pub struct ChannelEventConsumer {
    receiver: Arc<Mutex<mpsc::Receiver<DomainEvent>>>,
}

impl EventConsumer for ChannelEventConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        let receiver = Arc::clone(&self.receiver);
        Box::pin(stream::unfold(receiver, |rx| async move {
            let event = rx.lock().await.recv().await?;
            let envelope = EventEnvelope::new(event, Box::new(NoopAck));
            Some((Ok(envelope), rx))
        }))
    }
}

pub struct NoopEventPublisher;

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, _event: &DomainEvent) -> Result<(), DomainError> {
        Ok(())
    }
}

pub fn create_event_channel(
    config: EventPublisherConfig,
) -> (ChannelEventPublisher, ChannelEventConsumer) {
    let (tx, rx) = mpsc::channel(config.channel_buffer);
    (
        ChannelEventPublisher { sender: tx },
        ChannelEventConsumer {
            receiver: Arc::new(Mutex::new(rx)),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        events::DomainEvent,
        value_objects::{ExternalMetadataId, MovieId},
    };
    use futures::StreamExt;

    fn movie_discovered() -> DomainEvent {
        DomainEvent::MovieDiscovered {
            movie_id: MovieId::generate(),
            external_metadata_id: ExternalMetadataId::new("tt1234567".into()).unwrap(),
        }
    }

    #[tokio::test]
    async fn consumer_yields_published_events() {
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, consumer) = create_event_channel(config);

        publisher.publish(&movie_discovered()).await.unwrap();
        drop(publisher);

        let mut stream = consumer.consume();
        let envelope = stream.next().await.unwrap().unwrap();
        assert!(matches!(envelope.event, DomainEvent::MovieDiscovered { .. }));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn consumer_yields_multiple_events_in_order() {
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, consumer) = create_event_channel(config);

        publisher.publish(&movie_discovered()).await.unwrap();
        publisher.publish(&movie_discovered()).await.unwrap();
        drop(publisher);

        let mut stream = consumer.consume();
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        assert!(matches!(first.event, DomainEvent::MovieDiscovered { .. }));
        assert!(matches!(second.event, DomainEvent::MovieDiscovered { .. }));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn stream_ends_when_publisher_dropped() {
        let config = EventPublisherConfig { channel_buffer: 8 };
        let (publisher, consumer) = create_event_channel(config);
        drop(publisher);

        let mut stream = consumer.consume();
        assert!(stream.next().await.is_none());
    }
}
