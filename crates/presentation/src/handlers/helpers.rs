use application::ports::HtmlPageContext;
use domain::value_objects::UserId;

use crate::state::AppState;

pub(crate) async fn build_page_context(
    state: &AppState,
    user_id: Option<UserId>,
    csrf_token: String,
) -> HtmlPageContext {
    let uuid = user_id.as_ref().map(|u| u.value());
    let (user_email, is_admin) = if let Some(ref id) = user_id {
        let user = state.app_ctx.repos.user.find_by_id(id).await.ok().flatten();
        let email = user.as_ref().map(|u| u.email().value().to_string());
        let admin = user
            .as_ref()
            .map(|u| matches!(u.role(), domain::models::UserRole::Admin))
            .unwrap_or(false);
        (email, admin)
    } else {
        (None, false)
    };
    HtmlPageContext {
        user_email,
        user_id: uuid,
        is_admin,
        register_enabled: state.app_ctx.config.allow_registration,
        rss_url: "/feed.rss".to_string(),
        page_title: "Movies Diary".to_string(),
        canonical_url: state.app_ctx.config.base_url.clone(),
        csrf_token,
        page_rss_url: None,
    }
}
