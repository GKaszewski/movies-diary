use domain::errors::DomainError;
use domain::models::UserRole;
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::users::deps::AuthorizeAdminDeps;

pub async fn execute(
    deps: &AuthorizeAdminDeps,
    user_id: Uuid,
) -> Result<Option<bool>, DomainError> {
    let id = UserId::from_uuid(user_id);
    let found = deps.user.find_by_id(&id).await?;
    Ok(found.map(|user| matches!(user.role(), UserRole::Admin)))
}
