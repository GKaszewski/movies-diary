use async_trait::async_trait;

use domain::errors::DomainError;

use crate::diary::commands::LogReviewCommand;

#[async_trait]
pub trait ReviewLogger: Send + Sync {
    async fn log_review(&self, cmd: LogReviewCommand) -> Result<(), DomainError>;
}
