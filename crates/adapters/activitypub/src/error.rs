use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct Error(pub(crate) anyhow::Error);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl<T> From<T> for Error
where
    T: Into<anyhow::Error>,
{
    fn from(t: T) -> Self {
        Error(t.into())
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let msg = self.0.to_string();
        let status = if msg.contains("not found") {
            tracing::debug!(error = %msg, "AP: not found");
            (axum::http::StatusCode::NOT_FOUND, "Not found")
        } else if msg.contains("invalid") || msg.contains("bad") {
            tracing::debug!(error = %msg, "AP: bad request");
            (axum::http::StatusCode::BAD_REQUEST, "Bad request")
        } else {
            tracing::error!(error = %msg, "AP: internal error");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        };
        status.into_response()
    }
}
