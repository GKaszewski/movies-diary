use std::sync::Arc;

use application::context::AppContext;

use crate::ports::HtmlRenderer;

#[derive(Clone)]
pub struct AppState {
    pub app_ctx: AppContext,
    pub html_renderer: Arc<dyn HtmlRenderer>,
}
