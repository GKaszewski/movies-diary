use api_types::HtmlPageContext;
use domain::value_objects::UserId;

use crate::state::AppState;

pub(crate) fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

/// Error text for a classic-page `?error=` redirect. Infrastructure failures
/// are logged here — the redirect is their only exit, so nothing else would —
/// and shown to the user generically, never with internal detail.
pub(crate) fn encode_domain_error(e: &domain::errors::DomainError) -> String {
    match e {
        domain::errors::DomainError::InfrastructureError(_) => {
            tracing::error!("Internal error: {e}");
            encode_error("Something went wrong, please try again")
        }
        _ => encode_error(&e.to_string()),
    }
}

pub(crate) async fn build_page_context(
    state: &AppState,
    user_id: Option<UserId>,
    csrf_token: String,
) -> HtmlPageContext {
    let uuid = user_id.as_ref().map(|u| u.value());
    // Page chrome (email/admin badge, pending-follow badge) degrades rather than
    // blanking the page: a failed lookup or count logs a warning and falls back
    // to a zeroed `PageViewer`, same as the previous silent `.unwrap_or(0)` — now
    // logged and explicit, per the error policy in `get_page_viewer`'s doc comment.
    let (user_email, is_admin, pending_follow_count) = if let Some(ref id) = user_id {
        match application::users::get_page_viewer::execute(
            &state.app_ctx.deps.users.get_page_viewer,
            id.value(),
        )
        .await
        {
            Ok(viewer) => (viewer.email, viewer.is_admin, viewer.pending_follow_count),
            Err(e) => {
                tracing::warn!("get_page_viewer failed for {}: {e}", id.value());
                (None, false, 0)
            }
        }
    } else {
        (None, false, 0)
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
        pending_follow_count,
    }
}
