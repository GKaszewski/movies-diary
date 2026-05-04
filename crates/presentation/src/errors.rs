use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::errors::DomainError;

pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.0 {
            DomainError::InvalidRating { .. } => (StatusCode::BAD_REQUEST, self.0.to_string()),
            DomainError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            DomainError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            DomainError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            DomainError::InfrastructureError(_) => {
                tracing::error!("Internal Infrastructure Error: {:?}", self.0);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (status, error_message).into_response()
    }
}
