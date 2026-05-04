use domain::models::{DiaryEntry, collections::Paginated};

pub trait HtmlRenderer: Send + Sync {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>) -> Result<String, String>;
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry]) -> Result<String, String>;
}
