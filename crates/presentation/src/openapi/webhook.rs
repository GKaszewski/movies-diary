use api_types::{
    ConfirmWatchEntry, ConfirmWatchRequest, ConfirmWatchResponse, DismissWatchRequest,
    DismissWatchResponse, GenerateTokenRequest, GenerateTokenResponse, WatchQueueEntryDto,
    WebhookTokenDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::webhook::post_jellyfin_webhook,
        crate::handlers::webhook::post_plex_webhook,
        crate::handlers::webhook::post_generate_webhook_token,
        crate::handlers::webhook::get_webhook_tokens,
        crate::handlers::webhook::delete_webhook_token,
        crate::handlers::webhook::get_watch_queue,
        crate::handlers::webhook::post_confirm_watch_events,
        crate::handlers::webhook::post_dismiss_watch_events,
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
