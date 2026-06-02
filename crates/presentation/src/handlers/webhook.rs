use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use uuid::Uuid;

use api_types::{
    ConfirmWatchRequest, ConfirmWatchResponse, DismissWatchRequest, DismissWatchResponse,
    GenerateTokenRequest, GenerateTokenResponse, WatchQueueEntryDto, WebhookTokenDto,
};
use application::integrations::{
    commands::{
        ConfirmWatchEventsCommand, DismissWatchEventsCommand, GenerateWebhookTokenCommand,
        IngestWatchEventCommand, RevokeWebhookTokenCommand, WatchEventConfirmation,
    },
    confirm as confirm_watch_events, dismiss as dismiss_watch_events,
    generate_token as generate_webhook_token, get_queue as get_watch_queue,
    get_tokens as get_webhook_tokens, ingest as ingest_watch_event,
    queries::{GetWatchQueueQuery, GetWebhookTokensQuery},
    revoke_token as revoke_webhook_token,
};
use domain::models::WatchEventSource;

use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_string())
}

#[derive(serde::Deserialize, Default)]
pub struct WebhookTokenQuery {
    pub token: Option<String>,
}

fn extract_webhook_token(headers: &HeaderMap, query: &WebhookTokenQuery) -> Option<String> {
    extract_bearer_token(headers).or_else(|| query.token.clone())
}

// ── Webhook ingestion (no JWT, uses webhook bearer token) ─────────────────────

#[utoipa::path(
    post, path = "/api/v1/webhooks/jellyfin",
    request_body(content = String, description = "Jellyfin webhook JSON payload (SendAllProperties=true)", content_type = "application/json"),
    responses(
        (status = 200, description = "Event accepted or ignored"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid or missing webhook token"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_jellyfin_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<WebhookTokenQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let token = match extract_webhook_token(&headers, &query) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED,
    };

    let cmd = IngestWatchEventCommand {
        token,
        raw_payload: body.to_vec(),
        source: WatchEventSource::Jellyfin,
    };

    run_ingest(&state, cmd, &jellyfin::JellyfinParser).await
}

// ── Plex webhook (multipart form data with `payload` JSON field) ──────────────

#[utoipa::path(
    post, path = "/api/v1/webhooks/plex",
    request_body(content = String, description = "Plex webhook multipart form (payload field contains JSON)", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Event accepted or ignored"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid or missing webhook token"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_plex_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<WebhookTokenQuery>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let token = match extract_webhook_token(&headers, &query) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED,
    };

    let mut payload_bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("payload")
            && let Ok(bytes) = field.bytes().await
        {
            payload_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let raw_payload = match payload_bytes {
        Some(b) => b,
        None => return StatusCode::BAD_REQUEST,
    };

    let cmd = IngestWatchEventCommand {
        token,
        raw_payload,
        source: WatchEventSource::Plex,
    };

    run_ingest(&state, cmd, &plex::PlexParser).await
}

async fn run_ingest(
    state: &AppState,
    cmd: IngestWatchEventCommand,
    parser: &dyn domain::ports::MediaServerParser,
) -> StatusCode {
    match ingest_watch_event::execute(&state.app_ctx, cmd, parser).await {
        Ok(()) => StatusCode::OK,
        Err(e) => crate::errors::domain_error_status(&e),
    }
}

// ── Token management (JWT-authenticated) ──────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/settings/webhook-tokens",
    request_body = GenerateTokenRequest,
    responses(
        (status = 200, body = GenerateTokenResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_generate_webhook_token(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<GenerateTokenRequest>,
) -> Result<Json<GenerateTokenResponse>, ApiError> {
    let provider: WatchEventSource = req
        .provider
        .parse()
        .map_err(|e: String| domain::errors::DomainError::ValidationError(e))?;

    let cmd = GenerateWebhookTokenCommand {
        user_id: user.0.value(),
        provider: provider.clone(),
        label: req.label,
    };

    let result = generate_webhook_token::execute(&state.app_ctx, cmd).await?;

    let base_url = &state.app_ctx.config.base_url;
    let webhook_url = format!("{base_url}/api/v1/webhooks/{provider}");

    Ok(Json(GenerateTokenResponse {
        id: result.token.id().value().to_string(),
        token: result.token_plaintext,
        webhook_url,
    }))
}

#[utoipa::path(
    get, path = "/api/v1/settings/webhook-tokens",
    responses(
        (status = 200, body = Vec<WebhookTokenDto>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_webhook_tokens(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<WebhookTokenDto>>, ApiError> {
    let query = GetWebhookTokensQuery {
        user_id: user.0.value(),
    };
    let tokens = get_webhook_tokens::execute(&state.app_ctx, query).await?;

    let dtos = tokens
        .into_iter()
        .map(|t| WebhookTokenDto {
            id: t.id().value().to_string(),
            provider: t.provider().to_string(),
            label: t.label().map(String::from),
            created_at: t.created_at().to_string(),
            last_used_at: t.last_used_at().map(|d| d.to_string()),
        })
        .collect();

    Ok(Json(dtos))
}

#[utoipa::path(
    delete, path = "/api/v1/settings/webhook-tokens/{id}",
    params(("id" = Uuid, Path, description = "Webhook token ID")),
    responses(
        (status = 204, description = "Token revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Token not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_webhook_token(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let cmd = RevokeWebhookTokenCommand {
        user_id: user.0.value(),
        token_id: id,
    };
    revoke_webhook_token::execute(&state.app_ctx, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Watch queue (JWT-authenticated) ───────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/watch-queue",
    responses(
        (status = 200, body = Vec<WatchQueueEntryDto>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_watch_queue(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<WatchQueueEntryDto>>, ApiError> {
    let query = GetWatchQueueQuery {
        user_id: user.0.value(),
    };
    let events = get_watch_queue::execute(&state.app_ctx, query).await?;

    let dtos = events
        .into_iter()
        .map(|e| WatchQueueEntryDto {
            id: e.id().value().to_string(),
            title: e.title().to_string(),
            year: e.year(),
            movie_id: e.movie_id().map(|m| m.value().to_string()),
            source: e.source().to_string(),
            watched_at: e.watched_at().to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

#[utoipa::path(
    post, path = "/api/v1/watch-queue/confirm",
    request_body = ConfirmWatchRequest,
    responses(
        (status = 200, body = ConfirmWatchResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — not your watch event"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_confirm_watch_events(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<ConfirmWatchRequest>,
) -> Result<Json<ConfirmWatchResponse>, ApiError> {
    let cmd = ConfirmWatchEventsCommand {
        user_id: user.0.value(),
        confirmations: req
            .confirmations
            .into_iter()
            .map(|c| WatchEventConfirmation {
                watch_event_id: c.watch_event_id,
                rating: c.rating,
                comment: c.comment,
            })
            .collect(),
    };

    let confirmed = confirm_watch_events::execute(&state.app_ctx, cmd).await?;
    Ok(Json(ConfirmWatchResponse { confirmed }))
}

#[utoipa::path(
    post, path = "/api/v1/watch-queue/dismiss",
    request_body = DismissWatchRequest,
    responses(
        (status = 200, body = DismissWatchResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — not your watch event"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_dismiss_watch_events(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<DismissWatchRequest>,
) -> Result<Json<DismissWatchResponse>, ApiError> {
    let cmd = DismissWatchEventsCommand {
        user_id: user.0.value(),
        event_ids: req.event_ids,
    };

    let dismissed = dismiss_watch_events::execute(&state.app_ctx, cmd).await?;
    Ok(Json(DismissWatchResponse { dismissed }))
}
