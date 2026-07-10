use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventPublisher, ObjectStorage},
    value_objects::UserId,
};

use crate::users::{commands::UpdateProfileCommand, deps::UpdateProfileDeps};

async fn upload_image(
    storage: &dyn ObjectStorage,
    event_publisher: &dyn EventPublisher,
    user_id: &UserId,
    kind: &str,
    old_path: Option<&str>,
    new_bytes: Option<Vec<u8>>,
    content_type: Option<&str>,
) -> Result<Option<String>, DomainError> {
    let Some(bytes) = new_bytes else {
        return Ok(old_path.map(|s| s.to_string()));
    };

    let ct = content_type.unwrap_or("");
    if !["image/jpeg", "image/png", "image/webp"].contains(&ct) {
        return Err(DomainError::ValidationError(
            format!("{kind} must be jpeg, png, or webp"),
        ));
    }

    if let Some(old) = old_path {
        let _ = storage.delete(old).await;
    }

    let key = format!("{kind}/{}", user_id.value());
    let stored = storage.store(&key, &bytes).await?;

    if let Err(e) = event_publisher
        .publish(&DomainEvent::ImageStored {
            key: stored.clone(),
        })
        .await
    {
        tracing::warn!("failed to emit ImageStored for {kind} {stored}: {e}");
    }

    Ok(Some(stored))
}

pub async fn execute(
    deps: &UpdateProfileDeps,
    cmd: UpdateProfileCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);

    let user = deps
        .user
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;

    let storage = deps.object_storage.as_ref();
    let events = deps.event_publisher.as_ref();

    let new_avatar_path = upload_image(
        storage,
        events,
        &user_id,
        "avatars",
        user.avatar_path(),
        cmd.avatar_bytes,
        cmd.avatar_content_type.as_deref(),
    )
    .await?;

    let new_banner_path = upload_image(
        storage,
        events,
        &user_id,
        "banners",
        user.banner_path(),
        cmd.banner_bytes,
        cmd.banner_content_type.as_deref(),
    )
    .await?;

    let moved_to = cmd.also_known_as.as_deref().and_then(|new_url| {
        if user.also_known_as().map(|s| s != new_url).unwrap_or(true) {
            Some(new_url.to_string())
        } else {
            None
        }
    });

    deps.user
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

    deps.event_publisher
        .publish(&DomainEvent::UserUpdated {
            user_id: user_id.clone(),
        })
        .await?;

    if let Some(new_actor_url) = moved_to {
        let _ = deps
            .event_publisher
            .publish(&DomainEvent::UserAccountMoved {
                user_id,
                new_actor_url,
            })
            .await;
    }

    Ok(())
}

#[cfg(test)]
#[path = "tests/update_profile.rs"]
mod tests;
