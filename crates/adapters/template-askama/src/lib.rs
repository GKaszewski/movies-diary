use askama::Template;
use domain::models::{DiaryEntry, collections::Paginated};
use presentation::ports::HtmlRenderer; // Assuming you exposed the port

#[derive(Template)]
#[template(path = "diary.html")]
struct DiaryTemplate<'a> {
    entries: &'a [DiaryEntry],
    current_offset: u32,
    limit: u32,
    has_more: bool,
}

pub struct AskamaHtmlRenderer;

impl AskamaHtmlRenderer {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRenderer for AskamaHtmlRenderer {
    fn render_diary_page(&self, data: &Paginated<DiaryEntry>) -> Result<String, String> {
        let has_more = (data.offset + data.limit) < data.total_count as u32;

        let template = DiaryTemplate {
            entries: &data.items,
            current_offset: data.offset,
            limit: data.limit,
            has_more,
        };

        template.render().map_err(|e| e.to_string())
    }
}
