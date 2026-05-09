use std::fmt::{Display, Formatter};

use axum::http::StatusCode;

#[derive(Debug)]
pub struct Error(pub(crate) anyhow::Error, pub(crate) StatusCode);

impl Error {
    pub fn not_found(e: impl Into<anyhow::Error>) -> Self {
        Self(e.into(), StatusCode::NOT_FOUND)
    }

    pub fn bad_request(e: impl Into<anyhow::Error>) -> Self {
        Self(e.into(), StatusCode::BAD_REQUEST)
    }
}

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
        Error(t.into(), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = self.1;
        if status.is_server_error() {
            tracing::error!(error = %self.0, status = status.as_u16(), "federation error");
        } else {
            tracing::debug!(error = %self.0, status = status.as_u16(), "federation response");
        }
        let body = if status.is_server_error() {
            "internal server error".to_string()
        } else {
            self.0.to_string()
        };
        (status, body).into_response()
    }
}
