use std::sync::Arc;

#[cfg(feature = "nats")]
use anyhow::Context;
use domain::ports::{EventConsumer, EventPublisher};

use crate::db::DbPool;
use infra_wiring::EventBusBackend;

pub async fn create(
    db_pool: &DbPool,
) -> anyhow::Result<(Arc<dyn EventPublisher>, Arc<dyn EventConsumer>)> {
    match EventBusBackend::from_env()? {
        EventBusBackend::Db => {
            tracing::info!("event bus: DB queue");
            match db_pool {
                #[cfg(feature = "postgres")]
                DbPool::Postgres(pool) => Ok(
                    postgres_event_queue::PostgresEventQueue::create_channel(pool.clone()).await?,
                ),
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
