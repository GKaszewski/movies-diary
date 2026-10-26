use async_nats::{
    Client,
    jetstream::{self, consumer::pull, message::AckKind, stream::Config as StreamConfig},
};
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope, EventId, retry_backoff},
    ports::EventConsumer,
};
use futures::{
    StreamExt,
    stream::{self, BoxStream},
};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::{config::NatsConfig, payload::NatsEventMessage, subject::consumer_subject_filter};

// ── JetStream ack handle ─────────────────────────────────────────────────────

struct NatsJetStreamAckHandle {
    message: async_nats::jetstream::Message,
    event_id: Option<EventId>,
    max_deliver: i64,
}

struct NakPlan {
    delay: std::time::Duration,
    exhausted: bool,
}

fn nak_plan(delivered: i64, max_deliver: i64) -> NakPlan {
    NakPlan {
        delay: retry_backoff(u32::try_from(delivered.max(0)).unwrap_or(u32::MAX)),
        exhausted: max_deliver > 0 && delivered >= max_deliver,
    }
}

#[async_trait]
impl AckHandle for NatsJetStreamAckHandle {
    async fn ack(&self) -> Result<(), DomainError> {
        tracing::debug!(
            "acknowledging message with sequence {}",
            self.message.info().unwrap().stream_sequence
        );

        self.message
            .ack()
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn nack(&self) -> Result<(), DomainError> {
        let info = self
            .message
            .info()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let plan = nak_plan(info.delivered, self.max_deliver);
        if plan.exhausted {
            tracing::error!(
                event_id = ?self.event_id.map(|id| id.value()),
                sequence = info.stream_sequence,
                delivered = info.delivered,
                "event exhausted max deliveries; it remains in the stream — see JetStream max-deliveries advisory"
            );
        } else {
            tracing::debug!(
                sequence = info.stream_sequence,
                delivered = info.delivered,
                "nak with backoff {:?}",
                plan.delay
            );
        }

        self.message
            .ack_with(AckKind::Nak(Some(plan.delay)))
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn defer(&self, delay: std::time::Duration) -> Result<(), DomainError> {
        self.message
            .ack_with(AckKind::Nak(Some(delay)))
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

// ── Core NATS ack handle (no-op) ─────────────────────────────────────────────

struct NoopAck;

#[async_trait]
impl AckHandle for NoopAck {
    async fn ack(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn nack(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn defer(&self, _: std::time::Duration) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── Envelope construction helpers ────────────────────────────────────────────

fn decode_message(bytes: &[u8]) -> Result<(Option<EventId>, DomainEvent), DomainError> {
    let message: NatsEventMessage = serde_json::from_slice(bytes)
        .map_err(|e| DomainError::InfrastructureError(format!("deserialize: {e}")))?;
    let id = message
        .event_id
        .as_deref()
        .map(|s| {
            uuid::Uuid::parse_str(s)
                .map(EventId::from_uuid)
                .map_err(|e| DomainError::InfrastructureError(format!("event_id: {e}")))
        })
        .transpose()?;
    Ok((id, DomainEvent::try_from(message.payload)?))
}

fn decode_js(
    msg: async_nats::jetstream::Message,
    max_deliver: i64,
) -> Result<EventEnvelope, DomainError> {
    let (id, event) = decode_message(&msg.payload)?;
    Ok(EventEnvelope::new(
        id,
        event,
        Box::new(NatsJetStreamAckHandle {
            message: msg,
            event_id: id,
            max_deliver,
        }),
    ))
}

fn decode_core(msg: async_nats::Message) -> Result<EventEnvelope, DomainError> {
    let (id, event) = decode_message(&msg.payload)?;
    Ok(EventEnvelope::new(id, event, Box::new(NoopAck)))
}

// ── Channel-bridge shared by both consumers ──────────────────────────────────

type EnvelopeRx = Arc<Mutex<mpsc::Receiver<Result<EventEnvelope, DomainError>>>>;

fn consume_from_rx(rx: EnvelopeRx) -> BoxStream<'static, Result<EventEnvelope, DomainError>> {
    Box::pin(stream::unfold(rx, |rx| async move {
        let item = rx.lock().await.recv().await?;
        Some((item, rx))
    }))
}

const DEDUP_WINDOW: std::time::Duration = std::time::Duration::from_secs(60 * 60);

const ACK_WAIT: std::time::Duration = std::time::Duration::from_secs(600);

fn stream_config_update(existing: &StreamConfig) -> Option<StreamConfig> {
    (existing.duplicate_window != DEDUP_WINDOW).then(|| StreamConfig {
        duplicate_window: DEDUP_WINDOW,
        ..existing.clone()
    })
}

fn consumer_config_update(
    existing: &jetstream::consumer::Config,
    desired_max_deliver: i64,
) -> Option<jetstream::consumer::Config> {
    if existing.max_deliver == desired_max_deliver && existing.ack_wait == ACK_WAIT {
        return None;
    }
    Some(jetstream::consumer::Config {
        ack_wait: ACK_WAIT,
        max_deliver: desired_max_deliver,
        ..existing.clone()
    })
}

// ── JetStream consumer ────────────────────────────────────────────────────────

pub struct NatsJetStreamConsumer {
    rx: EnvelopeRx,
}

impl NatsJetStreamConsumer {
    pub async fn create(cfg: &NatsConfig, client: Client) -> anyhow::Result<Self> {
        let js = jetstream::new(client);

        let stream = js
            .get_or_create_stream(StreamConfig {
                name: cfg.stream_name.clone(),
                subjects: vec![consumer_subject_filter(&cfg.subject_prefix)],
                max_messages: 100_000,
                duplicate_window: DEDUP_WINDOW,
                ..Default::default()
            })
            .await?;
        if let Some(updated) = stream_config_update(&stream.cached_info().config) {
            tracing::warn!(
                stream = %cfg.stream_name,
                "existing JetStream stream has a different duplicate window; updating to {:?}",
                DEDUP_WINDOW
            );
            if let Err(e) = js.update_stream(updated).await {
                tracing::warn!(
                    "could not update stream {}: {e}; continuing. Fix manually: nats stream edit {} --dupe-window {}s",
                    cfg.stream_name,
                    cfg.stream_name,
                    DEDUP_WINDOW.as_secs()
                );
            }
        }

        let subject_filter = consumer_subject_filter(&cfg.subject_prefix);
        let consumer = stream
            .get_or_create_consumer(
                cfg.consumer_name.as_str(),
                pull::Config {
                    durable_name: Some(cfg.consumer_name.clone()),
                    filter_subject: subject_filter,
                    ack_wait: ACK_WAIT,
                    max_deliver: cfg.max_deliver,
                    ..Default::default()
                },
            )
            .await?;
        if let Some(updated) =
            consumer_config_update(&consumer.cached_info().config, cfg.max_deliver)
        {
            tracing::warn!(
                consumer = %cfg.consumer_name,
                "existing JetStream consumer has different max_deliver/ack_wait; updating to max_deliver={}",
                cfg.max_deliver
            );
            if let Err(e) = stream.update_consumer(updated).await {
                tracing::warn!(
                    "could not update consumer {}: {e}; continuing. Fix manually: nats consumer edit {} {} --max-deliver {}",
                    cfg.consumer_name,
                    cfg.stream_name,
                    cfg.consumer_name,
                    cfg.max_deliver
                );
            }
        }

        let (tx, rx) = mpsc::channel(128);

        let max_deliver = cfg.max_deliver;
        tokio::spawn(async move {
            loop {
                let mut messages = match consumer.messages().await {
                    Err(e) => {
                        tracing::error!("failed to fetch messages: {}", e);

                        let _ = tx
                            .send(Err(DomainError::InfrastructureError(e.to_string())))
                            .await;
                        return;
                    }
                    Ok(m) => m,
                };
                while let Some(result) = messages.next().await {
                    let envelope = result
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))
                        .and_then(|m| decode_js(m, max_deliver));

                    if tx.send(envelope).await.is_err() {
                        tracing::info!("consumer channel closed, stopping message processing");
                        return;
                    }

                    tracing::debug!("message sent to consumer channel");
                }
            }
        });

        Ok(Self {
            rx: Arc::new(Mutex::new(rx)),
        })
    }
}

impl EventConsumer for NatsJetStreamConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        consume_from_rx(Arc::clone(&self.rx))
    }
}

// ── Core NATS consumer ────────────────────────────────────────────────────────

pub struct NatsCoreConsumer {
    rx: EnvelopeRx,
}

impl NatsCoreConsumer {
    pub async fn create(cfg: &NatsConfig, client: Client) -> anyhow::Result<Self> {
        let subject = consumer_subject_filter(&cfg.subject_prefix);
        let mut subscriber = client.subscribe(subject).await?;

        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                let envelope = decode_core(msg);

                tracing::debug!("message received and decoded, sending to consumer channel");

                if tx.send(envelope).await.is_err() {
                    tracing::info!("consumer channel closed, stopping message processing");
                    break;
                }
            }
        });

        Ok(Self {
            rx: Arc::new(Mutex::new(rx)),
        })
    }
}

impl EventConsumer for NatsCoreConsumer {
    fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
        consume_from_rx(Arc::clone(&self.rx))
    }
}

fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<NatsJetStreamConsumer>();
    check::<NatsCoreConsumer>();
}

#[cfg(test)]
#[path = "tests/consumer.rs"]
mod tests;
