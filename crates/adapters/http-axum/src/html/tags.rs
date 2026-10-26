use axum::{
    extract::Path,
    response::{IntoResponse, Redirect},
};

pub async fn get_tag(Path(tag): Path<String>) -> impl IntoResponse {
    if tag.eq_ignore_ascii_case("moviesdiary") {
        Redirect::temporary("/")
    } else {
        Redirect::temporary(&format!("/?search={}", tag))
    }
}
