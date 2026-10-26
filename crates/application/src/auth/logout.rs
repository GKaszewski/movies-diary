use domain::errors::DomainError;

use crate::auth::deps::LogoutDeps;

pub async fn execute(deps: &LogoutDeps, refresh_token: &str) -> Result<(), DomainError> {
    deps.refresh_session.revoke(refresh_token).await
}
