use domain::models::UserRole;
use domain::value_objects::Email;

use crate::auth::commands::RegisterCommand;
use crate::auth::deps::RegisterDeps;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::users::deps::GetProfileDeps;
use crate::users::get_profile;
use crate::users::queries::{GetUserProfileQuery, ProfileView};

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

#[tokio::test]
async fn returns_profile_with_empty_stats() {
    let b = TestContextBuilder::new();
    let user_repo = b.user_repo.clone();
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "profile@test.com", "profuser").await;

    let email = Email::new("profile@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "hist@test.com", "histuser").await;

    let email = Email::new("hist@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::History,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "trends@test.com", "trendsuser").await;

    let email = Email::new("trends@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Trends,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: true,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "ratings@test.com", "ratingsuser").await;

    let email = Email::new("ratings@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Ratings,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Rating,
            search: None,
            is_own_profile: true,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "search@test.com", "searchuser").await;

    let email = Email::new("search@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: Some(10),
            offset: Some(0),
            sort_by: domain::models::FeedSortBy::Date,
            search: Some("blade".into()),
            is_own_profile: true,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };

    setup_user(&b, "other@test.com", "otheruser").await;

    let email = Email::new("other@test.com".into()).unwrap();
    let user = user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::models::FeedSortBy::Date,
            search: None,
            is_own_profile: false,
        },
    )
    .await
    .unwrap();

    assert!(result.pending_followers.is_empty());
}
