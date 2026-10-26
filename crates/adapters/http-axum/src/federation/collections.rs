use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

pub async fn get_followers_collection(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
        if state
            .app_ctx
            .ap_document
            .actor_is_gone(user_id)
            .await
            .unwrap_or(false)
        {
            return StatusCode::GONE.into_response();
        }
        let page = params.get("page").and_then(|p| p.parse::<u32>().ok());
        return match state
            .app_ctx
            .ap_document
            .followers_collection_json(user_id, page)
            .await
        {
            Ok(json) => (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/activity+json",
                )],
                json,
            )
                .into_response(),
            Err(e) => {
                tracing::error!("followers_collection_json error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }
    axum::response::Redirect::to(&format!("/users/{}/followers-list", user_id)).into_response()
}

pub async fn get_following_collection(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
        if state
            .app_ctx
            .ap_document
            .actor_is_gone(user_id)
            .await
            .unwrap_or(false)
        {
            return StatusCode::GONE.into_response();
        }
        let page = params.get("page").and_then(|p| p.parse::<u32>().ok());
        return match state
            .app_ctx
            .ap_document
            .following_collection_json(user_id, page)
            .await
        {
            Ok(json) => (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/activity+json",
                )],
                json,
            )
                .into_response(),
            Err(e) => {
                tracing::error!("following_collection_json error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }
    axum::response::Redirect::to(&format!("/users/{}/following-list", user_id)).into_response()
}
