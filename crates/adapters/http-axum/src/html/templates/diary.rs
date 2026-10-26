use super::{PageItem, filters};
use api_types::HtmlPageContext;
use askama::Template;
use domain::models::{DiaryEntry, FeedEntry, ReviewSource};

#[derive(Template)]
#[template(path = "diary.html")]
pub struct DiaryTemplate<'a> {
    pub entries: &'a [DiaryEntry],
    pub current_offset: u32,
    pub limit: u32,
    pub has_more: bool,
    pub ctx: &'a HtmlPageContext,
    pub page_items: Vec<PageItem>,
}

#[derive(Template)]
#[template(path = "new_review.html")]
pub struct NewReviewTemplate<'a> {
    pub error: Option<&'a str>,
    pub ctx: &'a HtmlPageContext,
}

#[derive(Template)]
#[template(path = "activity_feed.html")]
pub struct ActivityFeedTemplate<'a> {
    pub entries: &'a [FeedEntry],
    pub current_offset: u32,
    pub limit: u32,
    pub has_more: bool,
    pub ctx: &'a HtmlPageContext,
    pub page_items: Vec<PageItem>,
    pub filter: String,
    pub sort_by: String,
    pub search: String,
}

impl<'a> ActivityFeedTemplate<'a> {
    pub fn filter_qs(&self) -> String {
        let mut parts = vec![
            format!("filter={}", self.filter),
            format!("sort_by={}", self.sort_by),
        ];
        if !self.search.is_empty() {
            let encoded = self
                .search
                .replace(' ', "+")
                .replace('#', "%23")
                .replace('&', "%26")
                .replace('=', "%3D");
            parts.push(format!("search={}", encoded));
        }
        format!("&{}", parts.join("&"))
    }
}
