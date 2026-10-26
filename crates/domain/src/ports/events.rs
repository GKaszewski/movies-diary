use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope, EventId},
};

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

#[async_trait]
pub trait EventRelaySink: Send + Sync {
    async fn relay(&self, id: Option<&EventId>, event: &DomainEvent) -> Result<(), DomainError>;
}

pub trait EventConsumer: Send + Sync {
    fn consume(&self) -> futures::stream::BoxStream<'_, Result<EventEnvelope, DomainError>>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &'static str;
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}
