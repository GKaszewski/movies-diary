use super::filters;
use api_types::HtmlPageContext;
use askama::Template;
use domain::models::ReviewSource;

#[derive(Template)]
#[template(path = "movie_detail.html")]
pub struct MovieDetailTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub movie: &'a domain::models::Movie,
    pub stats: &'a domain::models::MovieStats,
    pub profile: Option<&'a domain::models::MovieProfile>,
    pub reviews: &'a [domain::models::FeedEntry],
    pub on_watchlist: bool,
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub histogram_max: u64,
}
