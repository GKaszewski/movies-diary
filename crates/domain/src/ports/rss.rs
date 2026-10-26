use crate::errors::DomainError;
use crate::models::DiaryEntry;

pub struct FeedChannel<'a> {
    pub title: &'a str,
    pub link_path: &'a str,
    pub self_path: &'a str,
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(
        &self,
        entries: &[DiaryEntry],
        channel: &FeedChannel<'_>,
    ) -> Result<String, DomainError>;
}
