use crate::models::DiaryEntry;

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String>;
}
