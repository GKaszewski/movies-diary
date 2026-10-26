use axum::{
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::state::AppState;

pub async fn actor_document_response(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
) -> Option<Response> {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !(accept.contains("application/activity+json") || accept.contains("application/ld+json")) {
        return None;
    }
    if state
        .app_ctx
        .ap_document
        .actor_is_gone(user_id)
        .await
        .unwrap_or(false)
    {
        return Some(StatusCode::GONE.into_response());
    }
    Some(
        match state
            .app_ctx
            .ap_document
            .actor_json(&user_id.to_string())
            .await
        {
            Ok(json) => {
                ([(header::CONTENT_TYPE, "application/activity+json")], json).into_response()
            }
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        },
    )
}
