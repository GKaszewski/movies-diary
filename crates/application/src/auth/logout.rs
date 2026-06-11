use std::sync::Arc;

use domain::{errors::DomainError, ports::RefreshSessionRepository};

pub async fn execute(
    refresh_session: Arc<dyn RefreshSessionRepository>,
    refresh_token: &str,
) -> Result<(), DomainError> {
    refresh_session.revoke(refresh_token).await
}

#[cfg(test)]
#[path = "tests/logout.rs"]
mod tests;
