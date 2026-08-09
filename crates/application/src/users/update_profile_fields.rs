use domain::{
    errors::DomainError, events::DomainEvent, models::UserProfile, value_objects::UserId,
};

use crate::users::commands::UpdateProfileFieldsCommand;
use crate::users::deps::UpdateProfileFieldsDeps;

pub async fn execute(
    deps: &UpdateProfileFieldsDeps,
    cmd: UpdateProfileFieldsCommand,
) -> Result<(), DomainError> {
    UserProfile::validate_custom_fields(&cmd.fields)?;
    let user_id = UserId::from_uuid(cmd.user_id);
    deps.profile_fields.set_fields(&user_id, cmd.fields).await?;
    deps.event_publisher
        .publish(&DomainEvent::UserUpdated { user_id })
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/update_profile_fields.rs"]
mod tests;
