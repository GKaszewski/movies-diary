use uuid::Uuid;

pub struct IngestWatchEventCommand {
    pub token: String,
    pub raw_payload: Vec<u8>,
    pub source: domain::models::WatchEventSource,
}

pub struct WatchEventConfirmation {
    pub watch_event_id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
}

pub struct ConfirmWatchEventsCommand {
    pub user_id: Uuid,
    pub confirmations: Vec<WatchEventConfirmation>,
}

pub struct DismissWatchEventsCommand {
    pub user_id: Uuid,
    pub event_ids: Vec<Uuid>,
}

pub struct GenerateWebhookTokenCommand {
    pub user_id: Uuid,
    pub provider: domain::models::WatchEventSource,
    pub label: Option<String>,
}

pub struct RevokeWebhookTokenCommand {
    pub user_id: Uuid,
    pub token_id: Uuid,
}
