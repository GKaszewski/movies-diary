use api_types::HtmlPageContext;
use askama::Template;

#[derive(Clone, Debug)]
pub struct WatchlistDisplayEntry {
    pub poster_url: Option<String>,
    pub movie_title: String,
    pub release_year: u16,
    pub movie_url: Option<String>,
    pub added_at: String,
    pub remove_url: Option<String>,
}

#[derive(Template)]
#[template(path = "watchlist.html")]
pub struct WatchlistTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub owner_id: uuid::Uuid,
    pub display_entries: &'a [WatchlistDisplayEntry],
    pub current_offset: u32,
    pub has_more: bool,
    pub limit: u32,
    pub is_owner: bool,
    pub error: Option<String>,
}
