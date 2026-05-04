use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventPublisher};
use tokio::sync::mpsc;

pub struct EventPublisherConfig {
    pub channel_buffer: usize,
}

impl EventPublisherConfig {
    pub fn from_env() -> Self {
        let channel_buffer = std::env::var("EVENT_CHANNEL_BUFFER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(128);
        Self { channel_buffer }
    }
}

pub struct ChannelEventPublisher {
    sender: mpsc::Sender<DomainEvent>,
}

#[async_trait]
impl EventPublisher for ChannelEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        self.sender
            .send(event.clone())
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

pub struct EventWorker {
    receiver: mpsc::Receiver<DomainEvent>,
}

impl EventWorker {
    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match &event {
                DomainEvent::ReviewLogged {
                    review_id,
                    movie_id,
                    user_id,
                    rating,
                    watched_at,
                } => {
                    tracing::info!(
                        review_id  = %review_id.value(),
                        movie_id   = %movie_id.value(),
                        user_id    = %user_id.value(),
                        rating     = rating.value(),
                        watched_at = %watched_at,
                        "event: review_logged"
                    );
                }
                DomainEvent::MovieDiscovered {
                    movie_id,
                    external_metadata_id,
                } => {
                    tracing::info!(
                        movie_id    = %movie_id.value(),
                        external_id = external_metadata_id.value(),
                        "event: movie_discovered"
                    );
                }
            }
        }
        tracing::info!("event worker shut down");
    }
}

pub fn create_event_channel(config: EventPublisherConfig) -> (ChannelEventPublisher, EventWorker) {
    let (tx, rx) = mpsc::channel(config.channel_buffer);
    (
        ChannelEventPublisher { sender: tx },
        EventWorker { receiver: rx },
    )
}
