use domain::errors::DomainError;
use domain::models::UserRole;
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::users::deps::AuthorizeAdminDeps;

/// Whether `user_id` is an admin, for the two extractors (`AdminApiUser`,
/// `AdminUser`) that gate admin-only routes.
///
/// Returns `Ok(None)` when the user row does not exist — a distinct success
/// value, not folded into `DomainError::NotFound`, specifically so callers can
/// tell "row missing" apart from "repository call failed" without depending on
/// which `DomainError` variant a lookup failure happens to produce. Each
/// extractor rejects those two cases differently (missing row is a 401/404,
/// a repository error is a 500) and that distinction must stay exact.
pub async fn execute(
    deps: &AuthorizeAdminDeps,
    user_id: Uuid,
) -> Result<Option<bool>, DomainError> {
    let id = UserId::from_uuid(user_id);
    let found = deps.user.find_by_id(&id).await?;
    Ok(found.map(|user| matches!(user.role(), UserRole::Admin)))
}

#[cfg(test)]
#[path = "tests/authorize_admin.rs"]
mod tests;
