use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use crate::{commands::UpdateProfileFieldsCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: UpdateProfileFieldsCommand) -> Result<(), DomainError> {
    if cmd.fields.len() > 4 {
        return Err(DomainError::ValidationError(
            "Maximum 4 profile fields allowed".into(),
        ));
    }
    let user_id = UserId::from_uuid(cmd.user_id);
    ctx.profile_fields_repository
        .set_fields(&user_id, cmd.fields)
        .await?;
    ctx.event_publisher
        .publish(&DomainEvent::UserUpdated { user_id })
        .await?;
    Ok(())
}
