use std::sync::Arc;

use domain::errors::DomainError;

use crate::{diary::commands::LogReviewCommand, ports::ReviewLogger};

pub async fn execute(
    review_logger: &Arc<dyn ReviewLogger>,
    cmd: LogReviewCommand,
) -> Result<(), DomainError> {
    review_logger.log_review(cmd).await
}

#[cfg(test)]
#[path = "tests/log_review.rs"]
mod tests;
