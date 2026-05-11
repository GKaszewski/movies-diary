use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};

use crate::state::AppState;

pub async fn get_image(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if key.starts_with("http://") || key.starts_with("https://") {
        return axum::response::Redirect::temporary(&key).into_response();
    }
    match state.app_ctx.image_storage.get(&key).await {
        Ok(bytes) => {
            let mime = infer::get(&bytes)
                .map(|t| t.mime_type())
                .unwrap_or("application/octet-stream");
            ([(header::CONTENT_TYPE, mime)], bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
