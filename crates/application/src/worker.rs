use std::sync::Arc;
use std::time::Duration;

use domain::{
    errors::DomainError,
    events::EventEnvelope,
    ports::{Begin, EventConsumer, EventHandler, ProcessedEventLedger},
};
use futures::StreamExt;
use tokio::sync::Semaphore;

const DEFAULT_CONCURRENCY: usize = 4;

pub struct WorkerService {
    consumer: Arc<dyn EventConsumer>,
    handlers: Vec<Arc<dyn EventHandler>>,
    ledger: Arc<dyn ProcessedEventLedger>,
    semaphore: Arc<Semaphore>,
}

impl WorkerService {
    pub fn new(
        consumer: Arc<dyn EventConsumer>,
        handlers: Vec<Arc<dyn EventHandler>>,
        ledger: Arc<dyn ProcessedEventLedger>,
    ) -> Result<Self, DomainError> {
        let mut seen = std::collections::HashSet::new();
        for handler in &handlers {
            if !seen.insert(handler.name()) {
                return Err(DomainError::ValidationError(format!(
                    "duplicate event handler name `{}`",
                    handler.name()
                )));
            }
        }
        Ok(Self {
            consumer,
            handlers,
            ledger,
            semaphore: Arc::new(Semaphore::new(DEFAULT_CONCURRENCY)),
        })
    }

    pub fn handler_names(&self) -> Vec<&'static str> {
        self.handlers.iter().map(|h| h.name()).collect()
    }

    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let handlers = Arc::new(self.handlers);
        let ledger = self.ledger;
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
                            let ledger = Arc::clone(&ledger);
                            tasks.spawn(async move {
                                dispatch(h, ledger, envelope).await;
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

const DEFER_MARGIN: Duration = Duration::from_secs(5);

async fn dispatch(
    handlers: Arc<Vec<Arc<dyn EventHandler>>>,
    ledger: Arc<dyn ProcessedEventLedger>,
    envelope: EventEnvelope,
) {
    let id = envelope.id().copied();
    let mut any_transient = false;
    let mut blocked_for: Option<Duration> = None;
    for handler in handlers.iter() {
        let name = handler.name();
        let mut claim = None;
        if let Some(id) = &id {
            match ledger.begin(id, name).await {
                Ok(Begin::Run(token)) => claim = Some(token),
                Ok(Begin::AlreadyDone) => {
                    tracing::debug!(handler = name, "already processed, skipping");
                    continue;
                }
                Ok(Begin::InProgressElsewhere { retry_after }) => {
                    tracing::debug!(handler = name, "running in another delivery, deferring");
                    blocked_for = Some(blocked_for.map_or(retry_after, |b| b.max(retry_after)));
                    continue;
                }
                Err(e) => {
                    tracing::warn!(handler = name, "ledger claim failed, nacking delivery: {e}");
                    any_transient = true;
                    continue;
                }
            }
        }
        let result = handler.handle(&envelope.event).await;
        match &result {
            Ok(()) => {
                if let (Some(id), Some(token)) = (&id, &claim)
                    && let Err(e) = ledger.complete(id, name, token).await
                {
                    tracing::warn!(handler = name, "completing ledger claim failed: {e}");
                }
            }
            Err(e) => {
                if let (Some(id), Some(token)) = (&id, &claim)
                    && let Err(le) = ledger.release(id, name, token).await
                {
                    tracing::warn!(handler = name, "releasing ledger claim failed: {le}");
                }
                if e.is_transient() {
                    tracing::warn!(
                        handler = name,
                        "transient handler error, nacking delivery (the backend logs if attempts are exhausted): {e}"
                    );
                    any_transient = true;
                } else {
                    tracing::warn!(
                        handler = name,
                        "permanent handler error (not retrying): {e}"
                    );
                }
            }
        }
    }
    if let Some(wait) = blocked_for {
        if let Err(e) = envelope.defer(wait + DEFER_MARGIN).await {
            tracing::error!("defer failed: {e}");
        }
    } else if any_transient {
        if let Err(e) = envelope.nack().await {
            tracing::error!("nack failed: {e}");
        }
    } else if let Err(e) = envelope.ack().await {
        tracing::error!("ack failed: {e}");
    }
}
