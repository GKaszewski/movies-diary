use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::models::collections::{PageParams, Paginated};
use domain::models::watchlist::{WatchlistEntry, WatchlistWithMovie};
use domain::models::{Movie, UserRole};
use domain::ports::WatchlistRepository;
use domain::value_objects::{Email, MovieId, MovieTitle, PosterPath, ReleaseYear, UserId};

use crate::auth::commands::RegisterCommand;
use crate::auth::register;
use crate::test_helpers::TestContextBuilder;
use crate::watchlist::get_page;
use crate::watchlist::queries::GetWatchlistQuery;

struct FakeWatchlistWithItems {
    user_id: UserId,
    items: Vec<WatchlistWithMovie>,
}

#[async_trait]
impl WatchlistRepository for FakeWatchlistWithItems {
    async fn add(&self, _entry: &WatchlistEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove(&self, _user_id: &UserId, _movie_id: &MovieId) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_if_present(
        &self,
        _user_id: &UserId,
        _movie_id: &MovieId,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_for_user(
        &self,
        user_id: &UserId,
        _page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        if user_id == &self.user_id {
            Ok(Paginated {
                total_count: self.items.len() as u64,
                limit: 20,
                offset: 0,
                items: self.items.clone(),
            })
        } else {
            Ok(Paginated {
                items: vec![],
                total_count: 0,
                limit: 20,
                offset: 0,
            })
        }
    }
    async fn contains(&self, _user_id: &UserId, _movie_id: &MovieId) -> Result<bool, DomainError> {
        Ok(false)
    }
}

#[tokio::test]
async fn returns_empty_for_local_user() {
    let ctx = TestContextBuilder::new().build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "wl@test.com".into(),
            username: "wluser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("wl@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_page::execute(
        &ctx,
        GetWatchlistQuery {
            user_id: uid,
            limit: None,
            offset: None,
        },
        true,
    )
    .await
    .unwrap();

    assert!(result.display_entries.is_empty());
}

#[tokio::test]
async fn returns_display_entries_for_local_user_with_items() {
    let ctx = TestContextBuilder::new().build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "wl2@test.com".into(),
            username: "wluser2".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("wl2@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let uid = user.id().value();

    let result = get_page::execute(
        &ctx,
        GetWatchlistQuery {
            user_id: uid,
            limit: Some(20),
            offset: Some(0),
        },
        true,
    )
    .await
    .unwrap();

    // InMemory get_for_user returns empty, but the local-user branch is exercised
    assert!(!result.has_more);
    assert_eq!(result.current_offset, 0);
}

#[tokio::test]
async fn returns_remote_watchlist_for_unknown_user() {
    let ctx = TestContextBuilder::new().build();

    let unknown_uid = uuid::Uuid::new_v4();

    let result = get_page::execute(
        &ctx,
        GetWatchlistQuery {
            user_id: unknown_uid,
            limit: None,
            offset: None,
        },
        false,
    )
    .await
    .unwrap();

    // NoopRemoteWatchlistRepository returns empty
    assert!(result.display_entries.is_empty());
    assert!(!result.has_more);
    assert_eq!(result.current_offset, 0);
}

#[tokio::test]
async fn maps_display_entries_for_owner() {
    let uid = uuid::Uuid::new_v4();
    let user_id = UserId::from_uuid(uid);
    let movie_id = MovieId::generate();

    let movie = Movie::from_persistence(
        movie_id.clone(),
        None,
        MovieTitle::new("Blade Runner".into()).unwrap(),
        ReleaseYear::new(1982).unwrap(),
        None,
        Some(PosterPath::new("poster123.jpg".into()).unwrap()),
    );
    let entry = WatchlistEntry::new(user_id.clone(), movie_id.clone());

    let fake_wl = Arc::new(FakeWatchlistWithItems {
        user_id: user_id.clone(),
        items: vec![WatchlistWithMovie {
            entry,
            movie: movie.clone(),
        }],
    });

    let ctx = TestContextBuilder::new()
        .with_watchlist(fake_wl as _)
        .build();

    // register user so find_by_id returns Some
    register::execute(
        &ctx,
        RegisterCommand {
            email: "wlmap@test.com".into(),
            username: "wlmapuser".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("wlmap@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let real_uid = user.id().value();

    // Rebuild with the real user_id in the fake
    let movie_id2 = MovieId::generate();
    let movie2 = Movie::from_persistence(
        movie_id2.clone(),
        None,
        MovieTitle::new("Blade Runner".into()).unwrap(),
        ReleaseYear::new(1982).unwrap(),
        None,
        Some(PosterPath::new("poster123.jpg".into()).unwrap()),
    );
    let entry2 = WatchlistEntry::new(UserId::from_uuid(real_uid), movie_id2.clone());

    let fake_wl2 = Arc::new(FakeWatchlistWithItems {
        user_id: UserId::from_uuid(real_uid),
        items: vec![WatchlistWithMovie {
            entry: entry2,
            movie: movie2.clone(),
        }],
    });

    let ctx2 = TestContextBuilder::new()
        .with_watchlist(fake_wl2 as _)
        .with_users(ctx.repos.user.clone())
        .build();

    let result = get_page::execute(
        &ctx2,
        GetWatchlistQuery {
            user_id: real_uid,
            limit: Some(20),
            offset: Some(0),
        },
        true,
    )
    .await
    .unwrap();

    assert_eq!(result.display_entries.len(), 1);
    let de = &result.display_entries[0];
    assert_eq!(de.movie_title, "Blade Runner");
    assert_eq!(de.release_year, 1982);
    assert_eq!(de.poster_url.as_deref(), Some("/images/poster123.jpg"));
    assert!(de.movie_url.is_some());
    assert!(de.remove_url.is_some()); // owner can remove
}

#[tokio::test]
async fn maps_display_entries_for_non_owner() {
    let ctx = TestContextBuilder::new().build();

    register::execute(
        &ctx,
        RegisterCommand {
            email: "wlno@test.com".into(),
            username: "wlnoowner".into(),
            password: "password123".into(),
            role: UserRole::Standard,
        },
    )
    .await
    .unwrap();

    let email = Email::new("wlno@test.com".into()).unwrap();
    let user = ctx.repos.user.find_by_email(&email).await.unwrap().unwrap();
    let real_uid = user.id().value();

    let movie_id = MovieId::generate();
    let movie = Movie::from_persistence(
        movie_id.clone(),
        None,
        MovieTitle::new("Alien".into()).unwrap(),
        ReleaseYear::new(1979).unwrap(),
        None,
        None,
    );
    let entry = WatchlistEntry::new(UserId::from_uuid(real_uid), movie_id.clone());

    let fake_wl = Arc::new(FakeWatchlistWithItems {
        user_id: UserId::from_uuid(real_uid),
        items: vec![WatchlistWithMovie {
            entry,
            movie: movie.clone(),
        }],
    });

    let ctx2 = TestContextBuilder::new()
        .with_watchlist(fake_wl as _)
        .with_users(ctx.repos.user.clone())
        .build();

    let result = get_page::execute(
        &ctx2,
        GetWatchlistQuery {
            user_id: real_uid,
            limit: Some(20),
            offset: Some(0),
        },
        false, // not owner
    )
    .await
    .unwrap();

    assert_eq!(result.display_entries.len(), 1);
    let de = &result.display_entries[0];
    assert_eq!(de.movie_title, "Alien");
    assert!(de.poster_url.is_none()); // no poster
    assert!(de.remove_url.is_none()); // not owner
}
