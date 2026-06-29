use async_trait::async_trait;

use domain::errors::DomainError;
use domain::models::DiaryEntry;

use crate::diary::commands::LogReviewCommand;

#[async_trait]
pub trait ReviewLogger: Send + Sync {
    async fn log_review(&self, cmd: LogReviewCommand) -> Result<(), DomainError>;
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String>;
}
