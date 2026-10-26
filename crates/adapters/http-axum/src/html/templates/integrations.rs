use api_types::HtmlPageContext;
use askama::Template;

#[derive(Template)]
#[template(path = "integrations.html")]
pub struct IntegrationsTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub tokens: &'a [WebhookTokenView],
    pub webhook_base_url: &'a str,
    pub new_token: Option<&'a str>,
}

pub struct WebhookTokenView {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Template)]
#[template(path = "watch_queue.html")]
pub struct WatchQueueTemplate<'a> {
    pub ctx: &'a HtmlPageContext,
    pub entries: &'a [WatchQueueDisplayEntry],
    pub error: Option<&'a str>,
}

pub struct WatchQueueDisplayEntry {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub source: String,
    pub watched_at: String,
    pub movie_url: Option<String>,
}
