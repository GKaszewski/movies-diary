use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};

use domain::value_objects::PosterPath;

use crate::state::AppState;

pub async fn get_poster(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    // If path is a remote URL, redirect directly instead of serving from local storage.
    if path.starts_with("http://") || path.starts_with("https://") {
        return axum::response::Redirect::temporary(&path).into_response();
    }

    let poster_path = match PosterPath::new(path) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    match state.app_ctx.poster_storage.get_poster(&poster_path).await {
        Ok(bytes) => {
            let mime = infer::get(&bytes)
                .map(|t| t.mime_type())
                .unwrap_or("application/octet-stream");
            ([(header::CONTENT_TYPE, mime)], bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
