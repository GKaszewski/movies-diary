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
    let event = DomainEvent::UserUpdated {
        user_id: user_id.clone(),
    };
    deps.profile_fields
        .set_fields(&user_id, cmd.fields, &[event])
        .await
}
