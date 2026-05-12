use std::sync::Arc;

#[cfg(feature = "nats")]
use anyhow::Context;
use domain::ports::{EventConsumer, EventPublisher};

use crate::db::DbPool;

#[derive(Clone, Copy)]
pub enum EventBusBackend {
    Db,
    #[cfg(feature = "nats")]
    Nats,
}

impl EventBusBackend {
    pub fn from_env() -> anyhow::Result<Self> {
        match std::env::var("EVENT_BUS_BACKEND")
            .unwrap_or_else(|_| "db".to_string())
            .as_str()
        {
            "db" => Ok(Self::Db),
            #[cfg(feature = "nats")]
            "nats" => Ok(Self::Nats),
            #[cfg(not(feature = "nats"))]
            "nats" => anyhow::bail!(
                "EVENT_BUS_BACKEND=nats requires the nats feature to be compiled in"
            ),
            other => anyhow::bail!("unknown EVENT_BUS_BACKEND={other}, expected 'db' or 'nats'"),
        }
    }
}

pub async fn create(
    db_pool: &DbPool,
) -> anyhow::Result<(Arc<dyn EventPublisher>, Arc<dyn EventConsumer>)> {
    match EventBusBackend::from_env()? {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            match db_pool {
                #[cfg(feature = "postgres")]
                DbPool::Postgres(pool) => {
                    Ok(postgres_event_queue::PostgresEventQueue::create_channel(pool.clone()).await?)
                }
                #[cfg(feature = "sqlite")]
                DbPool::Sqlite(pool) => {
                    Ok(sqlite_event_queue::SqliteEventQueue::create_channel(pool.clone()).await?)
                }
            }
        }
        #[cfg(feature = "nats")]
        EventBusBackend::Nats => {
            let cfg = nats::NatsConfig::from_env()
                .context("EVENT_BUS_BACKEND=nats requires NATS_URL to be set")?;
            tracing::info!("event bus: NATS ({})", cfg.url);
            Ok(nats::create_channel(cfg).await?)
        }
    }
}
