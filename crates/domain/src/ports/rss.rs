use crate::models::DiaryEntry;

/// Channel-level metadata for a rendered feed. Paths are site-relative
/// (e.g. `/users/{id}`); the renderer resolves them against its base URL.
pub struct FeedChannel<'a> {
    pub title: &'a str,
    /// Human-readable page the channel represents.
    pub link_path: &'a str,
    /// Where the feed itself is served, advertised as `atom:link rel="self"`.
    pub self_path: &'a str,
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(
        &self,
        entries: &[DiaryEntry],
        channel: &FeedChannel<'_>,
    ) -> Result<String, String>;
}
