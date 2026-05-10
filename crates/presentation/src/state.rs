use std::sync::Arc;

use application::context::AppContext;

use crate::ports::{HtmlRenderer, RssFeedRenderer};

#[derive(Clone)]
pub struct AppState {
    pub app_ctx: AppContext,
    pub html_renderer: Arc<dyn HtmlRenderer>,
    pub rss_renderer: Arc<dyn RssFeedRenderer>,
    #[cfg(feature = "federation")]
    pub ap_service: Arc<dyn activitypub::ActivityPubPort>,
    #[cfg(feature = "federation")]
    pub social_query: Arc<dyn domain::ports::SocialQueryPort>,
}
