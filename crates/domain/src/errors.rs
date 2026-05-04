use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Rating must be between 0 and {max}, but received {given}")]
    InvalidRating { max: u8, given: u8 },

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Business rule violation: {0}")]
    ValidationError(String),

    #[error("Infrastructure failure: {0}")]
    InfrastructureError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}
