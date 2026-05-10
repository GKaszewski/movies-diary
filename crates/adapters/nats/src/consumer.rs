use async_nats::{
    Client,
    jetstream::{self, consumer::pull, message::AckKind, stream::Config as StreamConfig},
};
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::{AckHandle, DomainEvent, EventEnvelope},
    ports::EventConsumer,
};
use futures::{
    StreamExt,
    stream::{self, BoxStream},
};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::{config::NatsConfig, payload::NatsEventPayload, subject::consumer_subject_filter};

// ── JetStream ack handle ─────────────────────────────────────────────────────

struct NatsJetStreamAckHandle {
    message: async_nats::jetstream::Message,
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
        tracing::debug!(
            "negatively acknowledging message with sequence {}",
            self.message.info().unwrap().stream_sequence
        );

        self.message
            .ack_with(AckKind::Nak(None))
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
}

// ── Envelope construction helpers ────────────────────────────────────────────

fn decode_js(msg: async_nats::jetstream::Message) -> Result<EventEnvelope, DomainError> {
    let payload: NatsEventPayload = serde_json::from_slice(&msg.payload)
        .map_err(|e| DomainError::InfrastructureError(format!("deserialize: {e}")))?;
    let event = DomainEvent::try_from(payload)?;
    Ok(EventEnvelope::new(
        event,
        Box::new(NatsJetStreamAckHandle { message: msg }),
    ))
}

fn decode_core(msg: async_nats::Message) -> Result<EventEnvelope, DomainError> {
    let payload: NatsEventPayload = serde_json::from_slice(&msg.payload)
        .map_err(|e| DomainError::InfrastructureError(format!("deserialize: {e}")))?;
    let event = DomainEvent::try_from(payload)?;
    Ok(EventEnvelope::new(event, Box::new(NoopAck)))
}

// ── Channel-bridge shared by both consumers ──────────────────────────────────

type EnvelopeRx = Arc<Mutex<mpsc::Receiver<Result<EventEnvelope, DomainError>>>>;

fn consume_from_rx(rx: EnvelopeRx) -> BoxStream<'static, Result<EventEnvelope, DomainError>> {
    Box::pin(stream::unfold(rx, |rx| async move {
        let item = rx.lock().await.recv().await?;
        Some((item, rx))
    }))
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
                ..Default::default()
            })
            .await?;

        let subject_filter = consumer_subject_filter(&cfg.subject_prefix);
        let consumer = stream
            .get_or_create_consumer(
                cfg.consumer_name.as_str(),
                pull::Config {
                    durable_name: Some(cfg.consumer_name.clone()),
                    filter_subject: subject_filter,
                    ..Default::default()
                },
            )
            .await?;

        let (tx, rx) = mpsc::channel(128);

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
                        .and_then(decode_js);

                    if tx.send(envelope).await.is_err() {
                        tracing::info!("consumer channel closed, stopping message processing");
                        return;
                    }

                    tracing::debug!("message sent to consumer channel");
                }
                // messages() stream ended (fetch expired in strict mode) — restart
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
