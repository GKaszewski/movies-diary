use std::sync::Arc;

use activitypub::ActivityPubPort;
use application::context::AppContext;

use crate::ports::{HtmlRenderer, RssFeedRenderer};

#[derive(Clone)]
pub struct AppState {
    pub app_ctx: AppContext,
    pub html_renderer: Arc<dyn HtmlRenderer>,
    pub rss_renderer: Arc<dyn RssFeedRenderer>,
    pub ap_service: Arc<dyn ActivityPubPort>,
}
