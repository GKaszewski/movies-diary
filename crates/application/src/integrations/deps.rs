use std::sync::Arc;

use domain::ports::{EventPublisher, WatchEventCommand, WatchEventQuery, WebhookTokenRepository};

pub struct IngestWatchEventDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
