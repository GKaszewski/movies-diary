use uuid::Uuid;

use domain::models::DiaryEntry;

pub struct HtmlPageContext {
    pub user_email: Option<String>,
    pub user_id: Option<Uuid>,
    pub is_admin: bool,
    pub register_enabled: bool,
    pub rss_url: String,
    pub page_title: String,
    pub canonical_url: String,
    pub csrf_token: String,
    pub page_rss_url: Option<String>,
}

impl HtmlPageContext {
    pub fn is_current_user(&self, id: Uuid) -> bool {
        self.user_id == Some(id)
    }
}

#[derive(Clone, Debug)]
pub struct WatchlistDisplayEntry {
    pub poster_url: Option<String>,
    pub movie_title: String,
    pub release_year: u16,
    pub movie_url: Option<String>,
    pub added_at: String,
    pub remove_url: Option<String>,
}

pub trait RssFeedRenderer: Send + Sync {
    fn render_feed(&self, entries: &[DiaryEntry], title: &str) -> Result<String, String>;
}
