use super::filters;
use api_types::HtmlPageContext;
use askama::Template;

#[derive(Template)]
#[template(path = "wrapup.html")]
pub struct WrapUpPageTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub report: &'a domain::models::wrapup::WrapUpReport,
    pub year_label: String,
    pub watch_time_display: String,
    pub rating_max: u32,
    pub genre_max: u32,
    pub rating_pcts: [f64; 5],
    pub genre_pcts: Vec<f64>,
}
