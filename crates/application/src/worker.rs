use std::sync::Arc;

use domain::{
    events::EventEnvelope,
    ports::{EventConsumer, EventHandler},
};
use futures::StreamExt;
use tokio::sync::Semaphore;

const DEFAULT_CONCURRENCY: usize = 4;

pub struct WorkerService {
    consumer: Arc<dyn EventConsumer>,
    handlers: Vec<Arc<dyn EventHandler>>,
    semaphore: Arc<Semaphore>,
}

impl WorkerService {
    pub fn new(consumer: Arc<dyn EventConsumer>, handlers: Vec<Arc<dyn EventHandler>>) -> Self {
        Self {
            consumer,
            handlers,
            semaphore: Arc::new(Semaphore::new(DEFAULT_CONCURRENCY)),
        }
    }

    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let handlers = Arc::new(self.handlers);
        let mut tasks = tokio::task::JoinSet::new();
        let mut stream = self.consumer.consume();

        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    tracing::info!("shutdown signal received, stopping event consumption");
                    break;
                }
                item = stream.next() => {
                    match item {
                        Some(Ok(envelope)) => {
                            tracing::info!(event = ?envelope.event, "received event");
                            let permit = self.semaphore.clone().acquire_owned().await;
                            let Ok(permit) = permit else { break };
                            let h = Arc::clone(&handlers);
                            tasks.spawn(async move {
                                dispatch(h, envelope).await;
                                drop(permit);
                            });
                        }
                        Some(Err(e)) => tracing::error!("event consumer error: {e}"),
                        None => break,
                    }
                }
            }
        }

        let in_flight = tasks.len();
        if in_flight > 0 {
            tracing::info!(in_flight, "draining in-flight tasks before shutdown");
        }
        while tasks.join_next().await.is_some() {}
        tracing::info!("worker shut down gracefully");
    }
}

async fn dispatch(handlers: Arc<Vec<Arc<dyn EventHandler>>>, envelope: EventEnvelope) {
    let mut any_transient = false;
    for handler in handlers.iter() {
        if let Err(e) = handler.handle(&envelope.event).await {
            if e.is_transient() {
                tracing::warn!("transient handler error, will retry: {e}");
                any_transient = true;
            } else {
                tracing::warn!("permanent handler error (not retrying): {e}");
            }
        }
    }
    if any_transient {
        if let Err(e) = envelope.nack().await {
            tracing::error!("nack failed: {e}");
        }
    } else if let Err(e) = envelope.ack().await {
        tracing::error!("ack failed: {e}");
    }
}

#[cfg(test)]
#[path = "tests/worker.rs"]
mod tests;
