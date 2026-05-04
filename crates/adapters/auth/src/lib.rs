use async_trait::async_trait;
use domain::{errors::DomainError, ports::AuthService, value_objects::UserId};

pub struct StubAuthService;

#[async_trait]
impl AuthService for StubAuthService {
    async fn validate_token(&self, _token: &str) -> Result<UserId, DomainError> {
        Err(DomainError::InfrastructureError(
            "auth service not implemented".into(),
        ))
    }
}
