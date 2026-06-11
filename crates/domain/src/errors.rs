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

    #[error("Forbidden: {0}")]
    Forbidden(String),
}

impl DomainError {
    pub fn is_transient(&self) -> bool {
        matches!(self, DomainError::InfrastructureError(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infrastructure_error_is_transient() {
        assert!(DomainError::InfrastructureError("network timeout".into()).is_transient());
    }

    #[test]
    fn not_found_is_not_transient() {
        assert!(!DomainError::NotFound("thing".into()).is_transient());
    }

    #[test]
    fn validation_error_is_not_transient() {
        assert!(!DomainError::ValidationError("bad input".into()).is_transient());
    }

    #[test]
    fn unauthorized_is_not_transient() {
        assert!(!DomainError::Unauthorized("token expired".into()).is_transient());
    }

    #[test]
    fn forbidden_is_not_transient() {
        assert!(!DomainError::Forbidden("no access".into()).is_transient());
    }

    #[test]
    fn invalid_rating_is_not_transient() {
        assert!(!DomainError::InvalidRating { max: 5, given: 9 }.is_transient());
    }
}
