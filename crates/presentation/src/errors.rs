use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::errors::DomainError;

pub fn domain_error_status(e: &DomainError) -> StatusCode {
    match e {
        DomainError::InvalidRating { .. } | DomainError::ValidationError(_) => {
            StatusCode::BAD_REQUEST
        }
        DomainError::NotFound(_) => StatusCode::NOT_FOUND,
        DomainError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        DomainError::Forbidden(_) => StatusCode::FORBIDDEN,
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
            DomainError::InfrastructureError(_) => {
                tracing::error!("Internal error: {:?}", self.0);
                (
                    status,
                    axum::Json(serde_json::json!({"error": "internal server error"})),
                )
                    .into_response()
            }
            _ => (
                status,
                axum::Json(serde_json::json!({"error": self.0.to_string()})),
            )
                .into_response(),
        }
    }
}
