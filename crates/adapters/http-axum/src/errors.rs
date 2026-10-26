use api_types::ErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::errors::DomainError;

pub const NOT_FOUND: &str = "NOT_FOUND";
pub const INVALID_INPUT: &str = "INVALID_INPUT";
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const INVALID_CREDENTIALS: &str = "INVALID_CREDENTIALS";
pub const FORBIDDEN: &str = "FORBIDDEN";
pub const CONFLICT: &str = "CONFLICT";
pub const RATE_LIMITED: &str = "RATE_LIMITED";
pub const INTERNAL: &str = "INTERNAL";

pub fn refuse(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        axum::Json(ErrorResponse {
            error: message.into(),
            code: code.to_string(),
        }),
    )
        .into_response()
}

pub fn domain_error_status(e: &DomainError) -> StatusCode {
    match e {
        DomainError::InvalidRating { .. } | DomainError::ValidationError(_) => {
            StatusCode::BAD_REQUEST
        }
        DomainError::NotFound(_) => StatusCode::NOT_FOUND,
        DomainError::Unauthorized(_) | DomainError::InvalidCredentials(_) => {
            StatusCode::UNAUTHORIZED
        }
        DomainError::Forbidden(_) => StatusCode::FORBIDDEN,
        DomainError::Conflict(_) => StatusCode::CONFLICT,
        DomainError::InfrastructureError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub fn domain_error_response(e: DomainError) -> Response {
    match &e {
        DomainError::InfrastructureError(_) => {
            tracing::error!("Internal error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        _ => (domain_error_status(&e), e.to_string()).into_response(),
    }
}

pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = domain_error_status(&self.0);
        match &self.0 {
            DomainError::InvalidRating { .. } | DomainError::ValidationError(_) => {
                refuse(status, INVALID_INPUT, self.0.to_string())
            }
            DomainError::NotFound(_) => refuse(status, NOT_FOUND, self.0.to_string()),
            DomainError::Unauthorized(_) => refuse(status, UNAUTHORIZED, self.0.to_string()),
            DomainError::InvalidCredentials(_) => {
                refuse(status, INVALID_CREDENTIALS, self.0.to_string())
            }
            DomainError::Forbidden(_) => refuse(status, FORBIDDEN, self.0.to_string()),
            DomainError::Conflict(_) => refuse(status, CONFLICT, self.0.to_string()),
            DomainError::InfrastructureError(_) => {
                tracing::error!("Internal error: {:?}", self.0);
                refuse(status, INTERNAL, "internal server error")
            }
        }
    }
}
