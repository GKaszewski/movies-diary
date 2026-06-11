use std::sync::Arc;

use domain::models::{ProfileField, User, UserProfile, UserRole};
use domain::ports::UserRepository;
use domain::testing::InMemoryUserRepository;
use domain::value_objects::{Email, PasswordHash, UserId, Username};
use uuid::Uuid;

use crate::{
    auth::{commands::RegisterCommand, deps::RegisterDeps, register},
    test_helpers::TestContextBuilder,
    users::{get_current_profile, queries::GetCurrentProfileQuery},
};

#[tokio::test]
async fn returns_profile_for_existing_user() {
    let users = InMemoryUserRepository::new();
    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let user_repo = b.user_repo.clone();
    let reg_deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };

    register::execute(
        &reg_deps,
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

    let profile = get_current_profile::execute(
        user_repo,
        GetCurrentProfileQuery {
            user_id: user.id().value(),
        },
    )
    .await
    .unwrap();

    assert_eq!(profile.username, "alice");
}

#[tokio::test]
async fn fails_for_nonexistent_user() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();

    let result = get_current_profile::execute(
        user_repo,
        GetCurrentProfileQuery {
            user_id: Uuid::new_v4(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn returns_profile_with_avatar_banner_and_fields() {
    let users = InMemoryUserRepository::new();
    let uid = UserId::generate();

    let user = User::from_persistence(
        uid.clone(),
        Email::new("full@example.com".into()).unwrap(),
        Username::new("fulluser".into()).unwrap(),
        PasswordHash::new("hashed".into()).unwrap(),
        UserRole::Standard,
        UserProfile {
            display_name: Some("Full Name".into()),
            bio: Some("My bio".into()),
            avatar_path: Some("avatars/abc123".into()),
            banner_path: Some("banners/def456".into()),
            also_known_as: None,
            profile_fields: vec![ProfileField {
                name: "Website".into(),
                value: "https://example.com".into(),
            }],
        },
    );
    users.store.lock().unwrap().insert(uid.value(), user);

    let b = TestContextBuilder::new().with_users(Arc::clone(&users) as _);
    let user_repo = b.user_repo.clone();

    let profile = get_current_profile::execute(
        user_repo,
        GetCurrentProfileQuery {
            user_id: uid.value(),
        },
    )
    .await
    .unwrap();

    assert_eq!(profile.username, "fulluser");
    assert_eq!(profile.display_name.as_deref(), Some("Full Name"));
    assert_eq!(profile.bio.as_deref(), Some("My bio"));
    assert_eq!(profile.avatar_path.as_deref(), Some("avatars/abc123"));
    assert_eq!(profile.banner_path.as_deref(), Some("banners/def456"));
    assert_eq!(profile.fields.len(), 1);
    assert_eq!(profile.fields[0].name, "Website");
    assert_eq!(profile.fields[0].value, "https://example.com");
}
