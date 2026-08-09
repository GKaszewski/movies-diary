use domain::errors::DomainError;
use domain::models::UserRole;
use domain::value_objects::Email;

use crate::auth::commands::RegisterCommand;
use crate::auth::deps::RegisterDeps;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::users::commands::UpdateProfileCommand;
use crate::users::deps::{GetLocalProfileDeps, UpdateProfileDeps};
use crate::users::get_local_profile;
use crate::users::queries::{GetUserProfileQuery, ProfileView};
use crate::users::update_profile;

async fn setup_user(b: &TestContextBuilder, email: &str, username: &str) {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(
        &deps,
        RegisterCommand {
            email: email.into(),
            username: username.into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();
}

fn deps(b: &TestContextBuilder) -> GetLocalProfileDeps {
    GetLocalProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query_unified.clone(),
        user: b.user_repo.clone(),
        instance: domain::value_objects::InstanceIdentity::new(b.config.base_url.clone()),
    }
}

fn unknown_query(user_id: uuid::Uuid) -> GetUserProfileQuery {
    GetUserProfileQuery {
        user_id,
        view: ProfileView::Recent,
        limit: None,
        offset: None,
        sort_by: domain::models::FeedSortBy::Date,
        search: None,
        is_own_profile: false,
        include_remote: false,
    }
}

/// `get_local_profile` is the local-only half of the former `get_profile` split
/// (ADR-0004's closed wart). Unlike the old tolerant behavior, a user id with no
/// local row must be `NotFound` here — the empty-string identity sentinel this
/// replaces cannot be represented in `LocalProfileData` at all.
#[tokio::test]
async fn get_local_profile_is_not_found_for_unknown_user() {
    let b = TestContextBuilder::new();
    let d = deps(&b);

    let unknown_id = uuid::Uuid::new_v4();
    let err = match get_local_profile::execute(&d, unknown_query(unknown_id)).await {
        Err(e) => e,
        Ok(_) => panic!("expected Err(NotFound) for an unknown user id, got Ok"),
    };
    assert!(matches!(err, DomainError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn returns_profile_with_empty_stats() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "profile@test.com", "profuser").await;

    let email = Email::new("profile@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.entries.is_some());
}

#[tokio::test]
async fn returns_history_view() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "hist@test.com", "histuser").await;

    let email = Email::new("hist@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::History,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.history.is_some());
    assert!(result.entries.is_none());
    assert!(result.trends.is_none());
}

#[tokio::test]
async fn returns_trends_view() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "trends@test.com", "trendsuser").await;

    let email = Email::new("trends@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Trends,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.trends.is_some());
    assert!(result.entries.is_none());
    assert!(result.history.is_none());
}

#[tokio::test]
async fn returns_ratings_view() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "ratings@test.com", "ratingsuser").await;

    let email = Email::new("ratings@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Ratings,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Rating,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.entries.is_some());
}

#[tokio::test]
async fn returns_recent_with_search() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "search@test.com", "searchuser").await;

    let email = Email::new("search@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: Some(10),
            offset: Some(0),
            sort_by: domain::models::FeedSortBy::Date,
            search: Some("blade".into()),
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.entries.is_some());
}

#[tokio::test]
async fn non_own_profile_skips_pending_followers() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "other@test.com", "otheruser").await;

    let email = Email::new("other@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: false,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert!(result.pending_followers.is_empty());
}

#[tokio::test]
async fn populates_handle_and_actor_url_for_a_local_user() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "handle@test.com", "gabriel").await;
    let email = Email::new("handle@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    // `InMemoryUserRepository::update_profile` now actually persists (previously a
    // no-op stub) — use it to give this test a real display_name/bio to assert,
    // rather than asserting `None` for a mapping that could just as easily be
    // wrong, inverted, or hardcoded.
    let update_deps = UpdateProfileDeps {
        user: user_repo.clone(),
        object_storage: b.object_storage.clone(),
        event_publisher: b.event_publisher.clone(),
    };
    update_profile::execute(
        &update_deps,
        UpdateProfileCommand {
            user_id: uid,
            display_name: Some("Gabriel K".into()),
            bio: Some("Movies and diaries.".into()),
            avatar_bytes: None,
            avatar_content_type: None,
            banner_bytes: None,
            banner_content_type: None,
            also_known_as: None,
        },
    )
    .await
    .unwrap();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.identity.username, "gabriel");
    assert_eq!(result.identity.handle, "@gabriel@localhost:3000");
    assert_eq!(
        result.identity.actor_url,
        format!("http://localhost:3000/users/{}", uid)
    );
    assert_eq!(result.identity.display_name, Some("Gabriel K".to_string()));
    assert_eq!(result.identity.bio, Some("Movies and diaries.".to_string()));
}

#[tokio::test]
async fn identity_display_name_and_bio_are_none_when_unset() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let d = deps(&b);

    setup_user(&b, "unset@test.com", "unsetuser").await;
    let email = Email::new("unset@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_local_profile::execute(
        &d,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
            include_remote: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.identity.display_name, None);
    assert_eq!(result.identity.bio, None);
}
