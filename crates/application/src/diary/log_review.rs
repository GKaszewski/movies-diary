use domain::errors::DomainError;

use crate::{context::AppContext, diary::commands::LogReviewCommand};

pub async fn execute(ctx: &AppContext, cmd: LogReviewCommand) -> Result<(), DomainError> {
    ctx.services.review_logger.log_review(cmd).await
}

#[cfg(test)]
#[path = "tests/log_review.rs"]
mod tests;
