use domain::{errors::DomainError, models::UserRole, value_objects::UserId};

use crate::users::deps::RequestAccountDeletionDeps;

pub async fn execute(
    deps: &RequestAccountDeletionDeps,
    actor: UserId,
    target: UserId,
    password: &str,
) -> Result<(), DomainError> {
    if target.is_service_actor() {
        return Err(DomainError::Forbidden(
            "the instance service account cannot be deleted".into(),
        ));
    }

    let actor_user = deps
        .user
        .find_by_id(&actor)
        .await?
        .ok_or_else(|| DomainError::Unauthorized("account is not active".into()))?;

    let is_self = actor == target;
    if !is_self && !matches!(actor_user.role(), UserRole::Admin) {
        return Err(DomainError::Forbidden(
            "only an admin can delete another account".into(),
        ));
    }

    if !deps
        .password_hasher
        .verify(password, actor_user.password_hash())
        .await?
    {
        return Err(DomainError::InvalidCredentials("incorrect password".into()));
    }

    let target_user = if is_self {
        actor_user
    } else {
        deps.user
            .find_by_id(&target)
            .await?
            .ok_or_else(|| DomainError::NotFound("User not found".into()))?
    };

    if matches!(target_user.role(), UserRole::Admin) && deps.user.count_active_admins().await? <= 1
    {
        return Err(DomainError::Conflict(
            "the last active admin cannot be deleted".into(),
        ));
    }

    deps.user
        .mark_deleted(
            &target,
            &[domain::events::DomainEvent::UserDeleted {
                user_id: target.clone(),
            }],
        )
        .await
}
