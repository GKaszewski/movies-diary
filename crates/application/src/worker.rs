use std::sync::Arc;

use domain::{
    events::EventEnvelope,
    ports::{EventConsumer, EventHandler},
};
use futures::StreamExt;

pub struct WorkerService {
    consumer: Arc<dyn EventConsumer>,
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl WorkerService {
    pub fn new(consumer: Arc<dyn EventConsumer>, handlers: Vec<Arc<dyn EventHandler>>) -> Self {
        Self { consumer, handlers }
    }

    pub async fn run(self) {
        let mut stream = self.consumer.consume();
        while let Some(result) = stream.next().await {
            match result {
                Ok(envelope) => {
                    tracing::info!(event = ?envelope.event, "received event");
                    self.dispatch(envelope).await;
                }
                Err(e) => tracing::error!("event consumer error: {e}"),
            }
        }
        tracing::info!("event stream ended, worker shutting down");
    }

    async fn dispatch(&self, envelope: EventEnvelope) {
        let mut all_ok = true;
        for handler in &self.handlers {
            if let Err(e) = handler.handle(&envelope.event).await {
                tracing::error!("event handler error: {e}");
                all_ok = false;
            }
        }
        let result = if all_ok {
            envelope.ack().await
        } else {
            envelope.nack().await
        };
        if let Err(e) = result {
            tracing::error!("ack/nack failed: {e}");
        }
    }
}

#[cfg(test)]
#[path = "tests/worker.rs"]
mod tests;
