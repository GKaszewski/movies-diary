use std::sync::Arc;

use domain::ports::{EventPublisher, WatchEventRepository, WebhookTokenRepository};

pub struct IngestWatchEventDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub watch_event: Arc<dyn WatchEventRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
