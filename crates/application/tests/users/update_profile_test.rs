use crate::support::TestContextBuilder;
use crate::support::{register_command, update_profile_command};
use std::sync::Arc;

use domain::events::DomainEvent;
use domain::ports::UserRepository;
use domain::testing::{InMemoryStore, NoopEventPublisher};
use uuid::Uuid;

use application::{
    auth::{deps::RegisterDeps, register},
    users::{commands::UpdateProfileCommand, deps::UpdateProfileDeps, update_profile},
};

async fn register_user(b: &TestContextBuilder, users: &Arc<InMemoryStore>) -> Uuid {
    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&reg_deps, register_command("alice@example.com", "alice"))
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
async fn updating_the_profile_records_the_change_and_its_event_together() {
    let users = InMemoryStore::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            display_name: Some("Alice W.".into()),
            ..update_profile_command(uid)
        },
    )
    .await
    .unwrap();

    let recorded = users.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0], DomainEvent::UserUpdated { .. }));
}

#[tokio::test]
async fn rejects_invalid_avatar_content_type() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            avatar_bytes: Some(vec![0u8; 10]),
            avatar_content_type: Some("image/gif".into()),
            ..update_profile_command(uid)
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn uploads_avatar() {
    let users = InMemoryStore::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            avatar_bytes: Some(vec![0xFFu8, 0xD8, 0xFF]),
            avatar_content_type: Some("image/jpeg".into()),
            ..update_profile_command(uid)
        },
    )
    .await
    .unwrap();

    assert!(
        users
            .recorded_events()
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        events
            .published()
            .iter()
            .any(|e| matches!(e, DomainEvent::ImageStored { .. }))
    );
}

#[tokio::test]
async fn uploads_banner() {
    let users = InMemoryStore::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            banner_bytes: Some(vec![0x89, 0x50, 0x4E]),
            banner_content_type: Some("image/png".into()),
            ..update_profile_command(uid)
        },
    )
    .await
    .unwrap();

    assert!(
        users
            .recorded_events()
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        events
            .published()
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
            display_name: Some("Ghost".into()),
            ..update_profile_command(Uuid::new_v4())
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_invalid_banner_content_type() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            banner_bytes: Some(vec![0u8; 10]),
            banner_content_type: Some("text/plain".into()),
            ..update_profile_command(uid)
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn text_only_update_emits_user_updated_no_image_stored() {
    let users = InMemoryStore::new();
    let events = NoopEventPublisher::new();
    let b = TestContextBuilder::new()
        .with_users(Arc::clone(&users) as _)
        .with_event_publisher(Arc::clone(&events) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            display_name: Some("Alice Updated".into()),
            bio: Some("Hello world".into()),
            ..update_profile_command(uid)
        },
    )
    .await
    .unwrap();

    assert!(
        users
            .recorded_events()
            .iter()
            .any(|e| matches!(e, DomainEvent::UserUpdated { .. }))
    );
    assert!(
        !events
            .published()
            .iter()
            .any(|e| matches!(e, DomainEvent::ImageStored { .. })),
        "text-only update should not emit ImageStored"
    );
}

#[tokio::test]
async fn changing_also_known_as_records_the_move_with_the_update() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let deps = UpdateProfileDeps {
        user: b.user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    let uid = register_user(&b, &users).await;

    update_profile::execute(
        &deps,
        UpdateProfileCommand {
            also_known_as: Some("https://other.example/users/alice".into()),
            ..update_profile_command(uid)
        },
    )
    .await
    .unwrap();

    let recorded = users.recorded_events();
    assert_eq!(recorded.len(), 2);
    assert!(matches!(recorded[0], DomainEvent::UserUpdated { .. }));
    assert!(matches!(recorded[1], DomainEvent::UserAccountMoved { .. }));
}

struct ProfileWriteRefusingUsers(Arc<InMemoryStore>);

#[async_trait::async_trait]
impl UserRepository for ProfileWriteRefusingUsers {
    async fn find_by_email(
        &self,
        e: &domain::value_objects::Email,
    ) -> Result<Option<domain::models::User>, domain::errors::DomainError> {
        self.0.find_by_email(e).await
    }
    async fn find_by_username(
        &self,
        u: &domain::value_objects::Username,
    ) -> Result<Option<domain::models::User>, domain::errors::DomainError> {
        self.0.find_by_username(u).await
    }
    async fn save(&self, u: &domain::models::User) -> Result<(), domain::errors::DomainError> {
        self.0.save(u).await
    }
    async fn find_by_id(
        &self,
        id: &domain::value_objects::UserId,
    ) -> Result<Option<domain::models::User>, domain::errors::DomainError> {
        self.0.find_by_id(id).await
    }
    async fn list_with_stats(
        &self,
    ) -> Result<Vec<domain::models::UserSummary>, domain::errors::DomainError> {
        self.0.list_with_stats().await
    }
    async fn update_profile(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::models::UserProfile,
        _: &[DomainEvent],
    ) -> Result<(), domain::errors::DomainError> {
        Err(domain::errors::DomainError::InfrastructureError(
            "refused".into(),
        ))
    }
    async fn mark_deleted(
        &self,
        id: &domain::value_objects::UserId,
        events: &[DomainEvent],
    ) -> Result<(), domain::errors::DomainError> {
        self.0.mark_deleted(id, events).await
    }
    async fn find_tombstoned_by_id(
        &self,
        id: &domain::value_objects::UserId,
    ) -> Result<Option<domain::models::User>, domain::errors::DomainError> {
        self.0.find_tombstoned_by_id(id).await
    }
    async fn is_username_retired(
        &self,
        u: &domain::value_objects::Username,
    ) -> Result<bool, domain::errors::DomainError> {
        self.0.is_username_retired(u).await
    }
    async fn count_active_admins(&self) -> Result<u64, domain::errors::DomainError> {
        self.0.count_active_admins().await
    }
}

#[tokio::test]
async fn a_refused_profile_update_propagates_and_records_nothing() {
    let users = InMemoryStore::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let uid = register_user(&b, &users).await;
    let deps = UpdateProfileDeps {
        user: Arc::new(ProfileWriteRefusingUsers(Arc::clone(&users))),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };

    let result = update_profile::execute(
        &deps,
        UpdateProfileCommand {
            display_name: Some("Alice W.".into()),
            ..update_profile_command(uid)
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(domain::errors::DomainError::InfrastructureError(m)) if m == "refused"
    ));
    assert!(users.recorded_events().is_empty());
}
