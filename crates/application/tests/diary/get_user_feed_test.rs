use crate::support::register_command;
use domain::testing::{test_movie_with_year, test_review};
use std::sync::Arc;

use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::{DiaryEntry, collections::Paginated};
use domain::testing::FakeDiaryQuery;
use domain::value_objects::{Email, UserId};

use crate::support::TestContextBuilder;
use application::auth::deps::RegisterDeps;
use application::auth::register;
use application::diary::deps::GetUserFeedDeps;
use application::diary::get_user_feed;

async fn setup_user(b: &TestContextBuilder, email: &str, username: &str) {
    let deps = RegisterDeps {
        user: b.user_repo.clone(),
        password_hasher: b.password_hasher.clone(),
        config: b.config.clone(),
    };
    register::execute(&deps, register_command(email, username))
        .await
        .unwrap();
}

#[tokio::test]
async fn user_feed_carries_author_and_entries() {
    let b = TestContextBuilder::new();
    setup_user(&b, "feed@test.com", "feeduser").await;

    let email = Email::new("feed@test.com".into()).unwrap();
    let user = b.user_repo.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let diary = FakeDiaryQuery::new();
    let movie = test_movie_with_year("Feed Movie", 2020);
    let review = test_review(movie.id(), &UserId::from_uuid(uid), 5);
    diary.set_diary_page(Paginated {
        items: vec![DiaryEntry::new(movie, review)],
        total_count: 1,
        limit: 50,
        offset: 0,
    });

    let deps = GetUserFeedDeps {
        user: b.user_repo.clone(),
        diary: Arc::clone(&diary) as _,
    };

    let feed = get_user_feed::execute(&deps, uid, 50).await.unwrap();

    assert_eq!(feed.author.display_name, "feed");
    assert_eq!(feed.entries.len(), 1);
}

#[tokio::test]
async fn user_feed_is_not_found_for_unknown_user() {
    let b = TestContextBuilder::new();
    let deps = GetUserFeedDeps {
        user: b.user_repo.clone(),
        diary: b.diary_repo.clone(),
    };

    let err = match get_user_feed::execute(&deps, Uuid::new_v4(), 50).await {
        Err(e) => e,
        Ok(_) => panic!("expected Err(NotFound) for an unknown user id, got Ok"),
    };
    assert!(matches!(err, DomainError::NotFound(_)));
}
