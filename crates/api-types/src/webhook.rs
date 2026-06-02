use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct GenerateTokenRequest {
    pub provider: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GenerateTokenResponse {
    pub id: String,
    pub token: String,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WebhookTokenDto {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WatchQueueEntryDto {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub movie_id: Option<String>,
    pub source: String,
    pub watched_at: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ConfirmWatchRequest {
    pub confirmations: Vec<ConfirmWatchEntry>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ConfirmWatchEntry {
    pub watch_event_id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ConfirmWatchResponse {
    pub confirmed: u32,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct DismissWatchRequest {
    pub event_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DismissWatchResponse {
    pub dismissed: u32,
}
