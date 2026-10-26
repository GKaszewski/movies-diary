use std::sync::Arc;

use domain::ports::{WatchEventCommand, WatchEventQuery, WebhookTokenRepository};

use crate::ports::ReviewLogger;

pub struct IngestWatchEventDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
}

pub struct ConfirmWatchEventsDeps {
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
    pub review_logger: Arc<dyn ReviewLogger>,
}

pub struct DismissWatchEventsDeps {
    pub watch_event_command: Arc<dyn WatchEventCommand>,
    pub watch_event_query: Arc<dyn WatchEventQuery>,
}

pub struct GenerateWebhookTokenDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
}

pub struct GetWatchQueueDeps {
    pub watch_event_query: Arc<dyn WatchEventQuery>,
}

pub struct GetWebhookTokensDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
}

pub struct RevokeWebhookTokenDeps {
    pub webhook_token: Arc<dyn WebhookTokenRepository>,
}
