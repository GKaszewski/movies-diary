use std::sync::Arc;

use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::UserProfile,
    ports::{EventPublisher, UserProfileFieldsRepository},
    value_objects::UserId,
};

use crate::users::commands::UpdateProfileFieldsCommand;

pub async fn execute(
    profile_fields: Arc<dyn UserProfileFieldsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    cmd: UpdateProfileFieldsCommand,
) -> Result<(), DomainError> {
    UserProfile::validate_custom_fields(&cmd.fields)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    profile_fields.set_fields(&user_id, cmd.fields).await?;
    event_publisher
        .publish(&DomainEvent::UserUpdated { user_id })
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/update_profile_fields.rs"]
mod tests;
