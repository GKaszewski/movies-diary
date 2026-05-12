use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::UserId,
};

use crate::{commands::UpdateProfileCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: UpdateProfileCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let user = ctx
        .user_repository
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    let new_avatar_path = if let Some(bytes) = cmd.avatar_bytes {
        let content_type = cmd.avatar_content_type.as_deref().unwrap_or("");
        if !["image/jpeg", "image/png", "image/webp"].contains(&content_type) {
            return Err(DomainError::ValidationError(
                "Avatar must be jpeg, png, or webp".into(),
            ));
        }
        if let Some(old_path) = user.avatar_path() {
            let _ = ctx.image_storage.delete(old_path).await;
        }
        let key = format!("avatars/{}", user_id.value());
        let stored = ctx.image_storage.store(&key, &bytes).await?;

        if let Err(e) = ctx.event_publisher
            .publish(&DomainEvent::ImageStored { key: stored.clone() })
            .await
        {
            tracing::warn!("failed to emit ImageStored for {stored}: {e}");
        }

        Some(stored)
    } else {
        user.avatar_path().map(|s| s.to_string())
    };

    ctx.user_repository
        .update_profile(&user_id, cmd.bio, new_avatar_path)
        .await?;

    ctx.event_publisher
        .publish(&DomainEvent::UserUpdated { user_id })
        .await?;

    Ok(())
}
