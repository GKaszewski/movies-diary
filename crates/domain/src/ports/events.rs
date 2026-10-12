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
    fn consume(&self) -> futures::stream::BoxStream<'_, Result<EventEnvelope, DomainError>>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}
