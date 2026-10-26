use api_types::{
    ConfirmWatchEntry, ConfirmWatchRequest, ConfirmWatchResponse, DismissWatchRequest,
    DismissWatchResponse, GenerateTokenRequest, GenerateTokenResponse, WatchQueueEntryDto,
    WebhookTokenDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::webhook::post_jellyfin_webhook,
        crate::api::webhook::post_plex_webhook,
        crate::api::webhook::post_generate_webhook_token,
        crate::api::webhook::get_webhook_tokens,
        crate::api::webhook::delete_webhook_token,
        crate::api::webhook::get_watch_queue,
        crate::api::webhook::post_confirm_watch_events,
        crate::api::webhook::post_dismiss_watch_events,
    ),
    components(schemas(
        GenerateTokenRequest,
        GenerateTokenResponse,
        WebhookTokenDto,
        WatchQueueEntryDto,
        ConfirmWatchRequest,
        ConfirmWatchEntry,
        ConfirmWatchResponse,
        DismissWatchRequest,
        DismissWatchResponse,
    ))
)]
pub struct WebhookDoc;
