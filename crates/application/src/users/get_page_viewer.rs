//! Backs page-chrome data: the nav bar's email/admin state and pending-follow
//! badge. Absorbs `html/helpers.rs::build_page_context`'s inline user lookup
//! and pending-follower count.
//!
//! **Error policy (deliberately no internal tolerance):** this function
//! propagates every error from `user` and `follow_graph` with `?`. It does not
//! itself decide to degrade — that's an http-adapter policy
//! (`build_page_context` logs a warning and falls back to a zeroed `PageViewer`
//! on `Err`). Page chrome degrading is a rendering decision, not a domain one.

use domain::{errors::DomainError, models::UserRole, value_objects::UserId};
use uuid::Uuid;

use crate::users::deps::GetPageViewerDeps;

pub struct PageViewer {
    pub email: Option<String>,
    pub is_admin: bool,
    pub pending_follow_count: usize,
}

pub async fn execute(deps: &GetPageViewerDeps, user_id: Uuid) -> Result<PageViewer, DomainError> {
    let uid = UserId::from_uuid(user_id);
    let user = deps
        .user
        .find_by_id(&uid)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("user {}", user_id)))?;

    // Reuse `FollowGraphQuery::count_pending_followers` directly — the same port
    // method `social::count_pending_followers::execute` wraps — rather than
    // re-deriving the count from raw follow rows a second time.
    let pending_follow_count = deps.follow_graph.count_pending_followers(&uid).await?;

    Ok(PageViewer {
        email: Some(user.email().value().to_string()),
        is_admin: matches!(user.role(), UserRole::Admin),
        pending_follow_count,
    })
}
