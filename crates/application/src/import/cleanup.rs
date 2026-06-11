use std::sync::Arc;

use domain::{errors::DomainError, ports::ImportSessionRepository};

pub async fn execute(import_session: Arc<dyn ImportSessionRepository>) -> Result<u64, DomainError> {
    import_session.delete_expired().await
}

#[cfg(test)]
#[path = "tests/cleanup.rs"]
mod tests;
