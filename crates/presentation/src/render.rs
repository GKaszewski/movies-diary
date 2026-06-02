use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub fn render_page(template: impl template_askama::askama::Template) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
