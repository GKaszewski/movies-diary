use async_nats::{jetstream, Client};
use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventPublisher};

use crate::{payload::NatsEventPayload, subject::event_to_subject};

enum PublisherMode {
    Core(Client),
    JetStream(jetstream::Context),
}

pub struct NatsEventPublisher {
    mode: PublisherMode,
    subject_prefix: String,
}

impl NatsEventPublisher {
    pub fn new_core(client: Client, subject_prefix: String) -> Self {
        Self { mode: PublisherMode::Core(client), subject_prefix }
    }

    pub fn new_jetstream(client: Client, subject_prefix: String) -> Self {
        Self { mode: PublisherMode::JetStream(jetstream::new(client)), subject_prefix }
    }
}

#[async_trait]
impl EventPublisher for NatsEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let subject = event_to_subject(&self.subject_prefix, event);
        let payload = serde_json::to_vec(&NatsEventPayload::from(event))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        match &self.mode {
            PublisherMode::Core(client) => client
                .publish(subject, payload.into())
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),

            PublisherMode::JetStream(js) => js
                .publish(subject, payload.into())
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                .await
                .map(|_| ())
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
        }
    }
}

fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<NatsEventPublisher>();
}
