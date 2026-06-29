use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope},
};

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

pub trait EventConsumer: Send + Sync {
    /// Returns a stream of event envelopes. Each envelope carries a domain event
    /// and an ack handle — callers ack after successful dispatch, nack on failure.
    /// Implementations decide transport (NATS, DB queue, in-memory channel).
    fn consume(&self) -> futures::stream::BoxStream<'_, Result<EventEnvelope, DomainError>>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}
