use std::sync::Arc;

use crate::context::AppContext;

use domain::ports::RssFeedRenderer;

#[derive(Clone)]
pub struct AppState {
    pub app_ctx: AppContext,
    pub rss_renderer: Arc<dyn RssFeedRenderer>,
}
