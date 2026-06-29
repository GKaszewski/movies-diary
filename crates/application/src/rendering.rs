use uuid::Uuid;

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
