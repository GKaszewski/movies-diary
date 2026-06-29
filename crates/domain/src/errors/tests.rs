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
