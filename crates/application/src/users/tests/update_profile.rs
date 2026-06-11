use std::sync::Arc;

use domain::events::DomainEvent;
use domain::models::UserRole;
use domain::ports::UserRepository;
use domain::testing::{InMemoryUserRepository, NoopEventPublisher};
use uuid::Uuid;

use crate::{
    auth::{commands::RegisterCommand, register},
    test_helpers::TestContextBuilder,
    users::{commands::UpdateProfileCommand, deps::UpdateProfileDeps, update_profile},
};

async fn register_user(
    ctx: &crate::context::AppContext,
    users: &Arc<InMemoryUserRepository>,
) -> Uuid {
    register::execute(
        ctx,
        RegisterCommand {
            email: "alice@example.com".into(),
            username: "alice".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let user = users
        .find_by_email(&domain::value_objects::Email::new("alice@example.com".into()).unwrap())
        .await
        .unwrap()
        .unwrap();
    user.id().value()
}

#[tokio::test]
async fn updates_display_name() {
    let users = InMemoryUserRepository::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: Some("Alice W.".into()),
            bio: None,
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
}

#[tokio::test]
async fn rejects_invalid_avatar_content_type() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: None,
            bio: None,
            avatar_bytes: Some(vec![0u8; 10]),
            avatar_content_type: Some("image/gif".into()),
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn uploads_avatar() {
    let users = InMemoryUserRepository::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: None,
            bio: None,
            avatar_bytes: Some(vec![0xFFu8, 0xD8, 0xFF]),
            avatar_content_type: Some("image/jpeg".into()),
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::ImageStored { .. }))
    );
}

#[tokio::test]
async fn uploads_banner() {
    let users = InMemoryUserRepository::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: None,
            bio: None,
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: Some(vec![0x89, 0x50, 0x4E]),
            banner_content_type: Some("image/png".into()),
            also_known_as: None,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::ImageStored { .. }))
    );
}

#[tokio::test]
async fn fails_for_nonexistent_user() {
    let b = TestContextBuilder::new();
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: Uuid::new_v4(),
            display_name: Some("Ghost".into()),
            bio: None,
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_invalid_banner_content_type() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: None,
            bio: None,
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: Some(vec![0u8; 10]),
            banner_content_type: Some("text/plain".into()),
            also_known_as: None,
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn text_only_update_emits_user_updated_no_image_stored() {
    let users = InMemoryUserRepository::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let ctx = b.build();

    let uid = register_user(&ctx, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: Some("Alice Updated".into()),
            bio: Some("Hello world".into()),
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        !published
            .iter()
            .any(|e| matches!(e, DomainEvent::ImageStored { .. })),
        "text-only update should not emit ImageStored"
    );
}
