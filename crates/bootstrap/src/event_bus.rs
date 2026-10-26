use std::sync::Arc;

use crate::outbox_relay::OutboxRelay;
#[cfg(feature = "nats")]
use anyhow::Context;
use domain::ports::{EventConsumer, EventPublisher, EventRelaySink};

#[derive(Clone, Copy)]
pub enum EventBusBackend {
    Db,
    #[cfg(feature = "nats")]
    Nats,
}

impl EventBusBackend {
    pub fn from_config(config: &config::EventsConfig) -> anyhow::Result<Self> {
        match config.bus_backend.as_str() {
            "db" => Ok(Self::Db),
            #[cfg(feature = "nats")]
            "nats" => Ok(Self::Nats),
            #[cfg(not(feature = "nats"))]
            "nats" => {
                anyhow::bail!("EVENT_BUS_BACKEND=nats requires the nats feature to be compiled in")
            }
            other => anyhow::bail!("unknown EVENT_BUS_BACKEND={other}, expected 'db' or 'nats'"),
        }
    }
}

pub async fn create_publisher(
    db_pool: &sqlx::SqlitePool,
    config: &config::AppConfig,
) -> anyhow::Result<Arc<dyn EventPublisher>> {
    tracing::info!("event publisher: outbox (DB queue)");
    sqlite::SqliteEventQueue::create_publisher(db_pool.clone(), &config.events).await
}

struct Wiring {
    publisher: Arc<dyn EventPublisher>,
    consumer: Arc<dyn EventConsumer>,
    relay: Option<OutboxRelay>,
}

fn compose(
    (publisher, outbox): (Arc<dyn EventPublisher>, Arc<dyn EventConsumer>),
    bus: Option<(Arc<dyn EventConsumer>, Arc<dyn EventRelaySink>)>,
) -> Wiring {
    match bus {
        None => Wiring {
            publisher,
            consumer: outbox,
            relay: None,
        },
        Some((consumer, sink)) => Wiring {
            publisher,
            consumer,
            relay: Some(OutboxRelay::new(outbox, sink)),
        },
    }
}

pub async fn create_channel(
    db_pool: &sqlx::SqlitePool,
    config: &config::AppConfig,
) -> anyhow::Result<(
    Arc<dyn EventPublisher>,
    Arc<dyn EventConsumer>,
    Option<OutboxRelay>,
)> {
    let outbox = sqlite::SqliteEventQueue::create_channel(db_pool.clone(), &config.events).await?;
    let wiring = match EventBusBackend::from_config(&config.events)? {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            compose(outbox, None)
        }
        #[cfg(feature = "nats")]
        EventBusBackend::Nats => {
            let cfg = nats::NatsConfig::from_config(&config.nats, &config.events)
                .context("EVENT_BUS_BACKEND=nats requires NATS_URL to be set")?;
            tracing::info!("event bus: NATS ({}), relaying the outbox", cfg.url);
            compose(outbox, Some(nats::create_bus(cfg).await?))
        }
    };
    Ok((wiring.publisher, wiring.consumer, wiring.relay))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        errors::DomainError,
        events::{DomainEvent, EventEnvelope, EventId},
    };
    use futures::stream::BoxStream;

    struct Nop;

    #[async_trait::async_trait]
    impl EventPublisher for Nop {
        async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl EventRelaySink for Nop {
        async fn relay(&self, _: Option<&EventId>, _: &DomainEvent) -> Result<(), DomainError> {
            Ok(())
        }
    }
    impl EventConsumer for Nop {
        fn consume(&self) -> BoxStream<'_, Result<EventEnvelope, DomainError>> {
            Box::pin(futures::stream::empty())
        }
    }

    fn outbox() -> (Arc<dyn EventPublisher>, Arc<dyn EventConsumer>) {
        (Arc::new(Nop), Arc::new(Nop))
    }

    #[test]
    fn db_mode_does_not_relay_and_consumes_the_outbox() {
        let (publisher, queue) = outbox();
        let queue_ptr = Arc::as_ptr(&queue);
        let wiring = compose((publisher, queue), None);
        assert!(wiring.relay.is_none());
        assert!(std::ptr::addr_eq(Arc::as_ptr(&wiring.consumer), queue_ptr));
    }

    #[test]
    fn bus_mode_relays_the_outbox_and_consumes_the_bus() {
        let (publisher, queue) = outbox();
        let bus_consumer: Arc<dyn EventConsumer> = Arc::new(Nop);
        let bus_ptr = Arc::as_ptr(&bus_consumer);
        let sink: Arc<dyn EventRelaySink> = Arc::new(Nop);

        let wiring = compose((publisher, queue), Some((bus_consumer, sink)));

        assert!(wiring.relay.is_some(), "relay built in bus mode");
        assert!(std::ptr::addr_eq(Arc::as_ptr(&wiring.consumer), bus_ptr));
    }
}
