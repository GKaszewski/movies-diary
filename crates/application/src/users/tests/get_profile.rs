use domain::models::UserRole;
use domain::value_objects::Email;

use crate::auth::commands::RegisterCommand;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::users::deps::GetProfileDeps;
use crate::users::get_profile;
use crate::users::queries::{GetUserProfileQuery, ProfileView};

fn default_deps() -> GetProfileDeps {
    let b = TestContextBuilder::new();
    GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    }
}

#[tokio::test]
async fn returns_profile_with_empty_stats() {
    let b = TestContextBuilder::new();
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "profile@test.com".into(),
            username: "profuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("profile@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::ports::FeedSortBy::Date,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "hist@test.com".into(),
            username: "histuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("hist@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::History,
            limit: None,
            offset: None,
            sort_by: domain::ports::FeedSortBy::Date,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "trends@test.com".into(),
            username: "trendsuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("trends@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Trends,
            limit: None,
            offset: None,
            sort_by: domain::ports::FeedSortBy::Date,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "ratings@test.com".into(),
            username: "ratingsuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("ratings@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Ratings,
            limit: None,
            offset: None,
            sort_by: domain::ports::FeedSortBy::Rating,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "search@test.com".into(),
            username: "searchuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("search@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: Some(10),
            offset: Some(0),
            sort_by: domain::ports::FeedSortBy::Date,
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
    let deps = GetProfileDeps {
        stats: b.stats_repo.clone(),
        diary: b.diary_repo.clone(),
        social_query: b.social_query.clone(),
    };
    let ctx = b.build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "other@test.com".into(),
            username: "otheruser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("other@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_profile::execute(
        &deps,
        GetUserProfileQuery {
            user_id: uid,
            view: ProfileView::Recent,
            limit: None,
            offset: None,
            sort_by: domain::ports::FeedSortBy::Date,
            search: None,
            is_own_profile: false,
        },
    )
    .await
    .unwrap();

    assert!(result.pending_followers.is_empty());
}
