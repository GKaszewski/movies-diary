use async_nats::{Client, HeaderMap, header::NATS_MESSAGE_ID, jetstream};
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{DomainEvent, EventId},
    ports::EventRelaySink,
};
use event_payload::EventPayload;

use crate::{payload::NatsEventMessage, subject::event_to_subject};

pub(crate) struct RelayMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    pub msg_id: Option<String>,
}

pub(crate) fn relay_message(
    prefix: &str,
    id: Option<&EventId>,
    event: &DomainEvent,
) -> Result<RelayMessage, DomainError> {
    let msg_id = id.map(|id| id.value().to_string());
    let payload = serde_json::to_vec(&NatsEventMessage {
        event_id: msg_id.clone(),
        payload: EventPayload::from(event),
    })
    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    Ok(RelayMessage {
        subject: event_to_subject(prefix, event),
        payload,
        msg_id,
    })
}

enum Transport {
    Core(Client),
    JetStream(jetstream::Context),
}

pub struct NatsRelaySink {
    transport: Transport,
    subject_prefix: String,
}

impl NatsRelaySink {
    pub fn new_core(client: Client, subject_prefix: String) -> Self {
        Self {
            transport: Transport::Core(client),
            subject_prefix,
        }
    }

    pub fn new_jetstream(client: Client, subject_prefix: String) -> Self {
        Self {
            transport: Transport::JetStream(jetstream::new(client)),
            subject_prefix,
        }
    }
}

fn infra(e: impl ToString) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl EventRelaySink for NatsRelaySink {
    async fn relay(&self, id: Option<&EventId>, event: &DomainEvent) -> Result<(), DomainError> {
        let message = relay_message(&self.subject_prefix, id, event)?;
        match &self.transport {
            Transport::Core(client) => client
                .publish(message.subject, message.payload.into())
                .await
                .map_err(infra),
            Transport::JetStream(js) => {
                let mut headers = HeaderMap::new();
                if let Some(msg_id) = &message.msg_id {
                    headers.insert(NATS_MESSAGE_ID, msg_id.as_str());
                }
                js.publish_with_headers(message.subject, headers, message.payload.into())
                    .await
                    .map_err(infra)?
                    .await
                    .map(|_| ())
                    .map_err(infra)
            }
        }
    }
}

fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<NatsRelaySink>();
}

#[cfg(test)]
#[path = "tests/relay.rs"]
mod tests;
