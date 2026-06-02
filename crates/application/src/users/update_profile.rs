use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use crate::{context::AppContext, users::commands::UpdateProfileCommand};

pub async fn execute(ctx: &AppContext, cmd: UpdateProfileCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let user = ctx
        .repos
        .user
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    // Handle avatar
    let new_avatar_path = if let Some(bytes) = cmd.avatar_bytes {
        let content_type = cmd.avatar_content_type.as_deref().unwrap_or("");
        if !["image/jpeg", "image/png", "image/webp"].contains(&content_type) {
            return Err(DomainError::ValidationError(
                "Avatar must be jpeg, png, or webp".into(),
            ));
        }
        if let Some(old_path) = user.avatar_path() {
            let _ = ctx.services.image_storage.delete(old_path).await;
        }
        let key = format!("avatars/{}", user_id.value());
        let stored = ctx.services.image_storage.store(&key, &bytes).await?;
        if let Err(e) = ctx
            .services
            .event_publisher
            .publish(&DomainEvent::ImageStored {
                key: stored.clone(),
            })
            .await
        {
            tracing::warn!("failed to emit ImageStored for avatar {stored}: {e}");
        }
        Some(stored)
    } else {
        user.avatar_path().map(|s| s.to_string())
    };

    // Handle banner
    let new_banner_path = if let Some(bytes) = cmd.banner_bytes {
        let content_type = cmd.banner_content_type.as_deref().unwrap_or("");
        if !["image/jpeg", "image/png", "image/webp"].contains(&content_type) {
            return Err(DomainError::ValidationError(
                "Banner must be jpeg, png, or webp".into(),
            ));
        }
        if let Some(old_path) = user.banner_path() {
            let _ = ctx.services.image_storage.delete(old_path).await;
        }
        let key = format!("banners/{}", user_id.value());
        let stored = ctx.services.image_storage.store(&key, &bytes).await?;
        if let Err(e) = ctx
            .services
            .event_publisher
            .publish(&DomainEvent::ImageStored {
                key: stored.clone(),
            })
            .await
        {
            tracing::warn!("failed to emit ImageStored for banner {stored}: {e}");
        }
        Some(stored)
    } else {
        user.banner_path().map(|s| s.to_string())
    };

    ctx.repos
        .user
        .update_profile(
            &user_id,
            &domain::models::UserProfile {
                display_name: cmd.display_name,
                bio: cmd.bio,
                avatar_path: new_avatar_path,
                banner_path: new_banner_path,
                also_known_as: cmd.also_known_as,
                profile_fields: vec![],
            },
        )
        .await?;

    ctx.services
        .event_publisher
        .publish(&DomainEvent::UserUpdated { user_id })
        .await?;

    Ok(())
}
