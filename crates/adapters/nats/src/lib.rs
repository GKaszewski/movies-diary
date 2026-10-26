mod config;
mod consumer;
mod payload;
mod relay;
mod subject;

pub use config::{NatsConfig, NatsMode};
pub use consumer::{NatsCoreConsumer, NatsJetStreamConsumer};
pub use relay::NatsRelaySink;

use std::sync::Arc;

use domain::ports::{EventConsumer, EventRelaySink};

pub async fn create_bus(
    cfg: NatsConfig,
) -> anyhow::Result<(Arc<dyn EventConsumer>, Arc<dyn EventRelaySink>)> {
    let client = async_nats::connect(&cfg.url).await?;

    let (consumer, sink): (Arc<dyn EventConsumer>, Arc<dyn EventRelaySink>) = match cfg.mode {
        NatsMode::Core => {
            tracing::info!(
                "core NATS mode: no broker-side dedup or redelivery; relying on consumer ledger"
            );
            (
                Arc::new(NatsCoreConsumer::create(&cfg, client.clone()).await?),
                Arc::new(NatsRelaySink::new_core(client, cfg.subject_prefix)),
            )
        }
        NatsMode::JetStream => (
            Arc::new(NatsJetStreamConsumer::create(&cfg, client.clone()).await?),
            Arc::new(NatsRelaySink::new_jetstream(client, cfg.subject_prefix)),
        ),
    };

    tracing::info!("NATS bus created (mode: {:?})", cfg.mode);
    Ok((consumer, sink))
}
