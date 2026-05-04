use domain::models::{DiaryEntry, collections::Paginated};

pub trait HtmlRenderer: Send + Sync {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>) -> Result<String, String>;
}
