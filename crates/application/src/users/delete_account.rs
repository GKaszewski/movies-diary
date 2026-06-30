use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use crate::users::deps::UpdateProfileDeps;

pub async fn execute(deps: &UpdateProfileDeps, user_id: uuid::Uuid) -> Result<(), DomainError> {
    let uid = UserId::from_uuid(user_id);

    deps.user
        .find_by_id(&uid)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    // Notify federation peers before any data is removed so they can process the tombstone.
    deps.event_publisher
        .publish(&DomainEvent::UserDeleted { user_id: uid })
        .await?;

    Ok(())
}
