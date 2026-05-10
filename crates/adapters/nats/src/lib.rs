mod config;
mod consumer;
mod payload;
mod publisher;
mod subject;

pub use config::{NatsConfig, NatsMode};
pub use consumer::{NatsCoreConsumer, NatsJetStreamConsumer};
pub use publisher::NatsEventPublisher;

use std::sync::Arc;

use domain::ports::{EventConsumer, EventPublisher};

pub async fn create_publisher(cfg: NatsConfig) -> anyhow::Result<Arc<dyn EventPublisher>> {
    let client = async_nats::connect(&cfg.url).await?;
    let publisher: Arc<dyn EventPublisher> = match cfg.mode {
        NatsMode::Core => Arc::new(NatsEventPublisher::new_core(client, cfg.subject_prefix)),
        NatsMode::JetStream => Arc::new(NatsEventPublisher::new_jetstream(
            client,
            cfg.subject_prefix,
        )),
    };

    tracing::info!("NATS publisher created (mode: {:?})", cfg.mode);
    Ok(publisher)
}

pub async fn create_channel(
    cfg: NatsConfig,
) -> anyhow::Result<(Arc<dyn EventPublisher>, Arc<dyn EventConsumer>)> {
    let client = async_nats::connect(&cfg.url).await?;

    let publisher: Arc<dyn EventPublisher> = match cfg.mode {
        NatsMode::Core => Arc::new(NatsEventPublisher::new_core(
            client.clone(),
            cfg.subject_prefix.clone(),
        )),
        NatsMode::JetStream => Arc::new(NatsEventPublisher::new_jetstream(
            client.clone(),
            cfg.subject_prefix.clone(),
        )),
    };

    let consumer: Arc<dyn EventConsumer> = match cfg.mode {
        NatsMode::Core => Arc::new(NatsCoreConsumer::create(&cfg, client).await?),
        NatsMode::JetStream => Arc::new(NatsJetStreamConsumer::create(&cfg, client).await?),
    };

    tracing::info!("NATS channel created (mode: {:?})", cfg.mode);
    Ok((publisher, consumer))
}
