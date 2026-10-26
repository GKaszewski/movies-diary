use std::{sync::Arc, time::Duration};

use domain::{
    events::{EventEnvelope, retry_backoff},
    ports::{EventConsumer, EventRelaySink},
};
use futures::StreamExt;
use tokio::sync::watch;

pub struct OutboxRelay {
    source: Arc<dyn EventConsumer>,
    sink: Arc<dyn EventRelaySink>,
    backoff: fn(u32) -> Duration,
}

impl OutboxRelay {
    pub fn new(source: Arc<dyn EventConsumer>, sink: Arc<dyn EventRelaySink>) -> Self {
        Self {
            source,
            sink,
            backoff: retry_backoff,
        }
    }

    pub fn with_backoff(mut self, backoff: fn(u32) -> Duration) -> Self {
        self.backoff = backoff;
        self
    }

    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut rows = self.source.consume();
        let mut sink_failures: u32 = 0;
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => {
                    tracing::info!("outbox relay: shutdown signal received, stopping");
                    return;
                }
                next = rows.next() => match next {
                    Some(Ok(envelope)) => {
                        let pause = relay_envelope(
                            envelope,
                            self.sink.as_ref(),
                            &mut sink_failures,
                            self.backoff,
                        )
                        .await;
                        if let Some(pause) = pause {
                            tokio::select! {
                                _ = tokio::time::sleep(pause) => {}
                                _ = shutdown.changed() => {
                                    tracing::info!("outbox relay: shutdown signal received, stopping");
                                    return;
                                }
                            }
                        }
                    }
                    Some(Err(e)) => tracing::error!("outbox relay: skipped undecodable row: {e}"),
                    None => {
                        tracing::error!("outbox relay: source closed");
                        return;
                    }
                },
            }
        }
    }
}

async fn relay_envelope(
    envelope: EventEnvelope,
    sink: &dyn EventRelaySink,
    sink_failures: &mut u32,
    backoff: fn(u32) -> Duration,
) -> Option<Duration> {
    match sink.relay(envelope.id(), &envelope.event).await {
        Ok(()) => {
            *sink_failures = 0;
            if let Err(e) = envelope.ack().await {
                tracing::error!("outbox relay: published but could not mark done: {e}");
            }
            None
        }
        Err(e) => {
            *sink_failures = sink_failures.saturating_add(1);
            let wait = backoff(*sink_failures);
            tracing::warn!(
                consecutive_failures = *sink_failures,
                "outbox relay: bus unavailable, deferring row {wait:?} without spending an attempt: {e}"
            );
            if let Err(e) = envelope.defer(wait).await {
                tracing::error!("outbox relay: could not defer: {e}");
            }
            Some(wait)
        }
    }
}
