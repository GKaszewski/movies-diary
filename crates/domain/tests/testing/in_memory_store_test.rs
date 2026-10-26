use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use domain::errors::DomainError;
use domain::models::{
    CastMember, CrewMember, Genre, Keyword, Movie, MovieProfile, ProfileField, RefreshSession,
    UserSettings, WatchEvent, WatchEventSource, WatchlistEntry, WebhookToken,
};
use domain::ports::{
    GoalCommand, MovieCommand, MovieProfileRepository, RefreshSessionRepository, ReviewRepository,
    SocialCommand, UserProfileFieldsRepository, UserRepository, UserSettingsRepository,
    WatchEventCommand, WatchEventQuery, WatchlistRepository, WebhookTokenRepository,
};
use domain::testing::{InMemoryStore, MOVIE_CASCADE_TABLES, USER_CASCADE_TABLES};
use domain::testing::{
    test_goal, test_movie, test_movie_with_external_id, test_review, test_user, test_watched_at,
};
use domain::value_objects::{ExternalMetadataId, FollowTarget, MovieId, SocialIdentity, UserId};

const REMOTE_ACTOR: &str = "https://remote.example/users/ann";

fn movie_profile(movie_id: &MovieId) -> MovieProfile {
    MovieProfile {
        movie_id: movie_id.clone(),
        tmdb_id: 603,
        imdb_id: None,
        overview: None,
        tagline: None,
        runtime_minutes: None,
        budget_usd: None,
        revenue_usd: None,
        vote_average: None,
        vote_count: None,
        original_language: None,
        collection_name: None,
        genres: vec![Genre {
            tmdb_id: 1,
            name: "Sci-Fi".into(),
        }],
        keywords: vec![Keyword {
            tmdb_id: 2,
            name: "simulation".into(),
        }],
        cast: vec![CastMember {
            tmdb_person_id: 3,
            name: "Actor".into(),
            character: "Role".into(),
            billing_order: 0,
            profile_path: None,
        }],
        crew: vec![CrewMember {
            tmdb_person_id: 4,
            name: "Director".into(),
            job: "Director".into(),
            department: "Directing".into(),
            profile_path: None,
        }],
        enriched_at: Utc::now(),
    }
}

async fn seed_user_with_dependents(store: &Arc<InMemoryStore>) -> (UserId, Movie) {
    let user = test_user("alice");
    let user_id = user.id().clone();
    let other = test_user("bob");
    let movie = test_movie("Heat");
    UserRepository::save(&**store, &user).await.unwrap();
    UserRepository::save(&**store, &other).await.unwrap();
    store.upsert_movie(&movie, &[]).await.unwrap();

    store
        .add(
            &WatchlistEntry::new(user_id.clone(), movie.id().clone()),
            &[],
        )
        .await
        .unwrap();
    GoalCommand::save(&**store, &test_goal(&user_id, 2024, 50), &[])
        .await
        .unwrap();
    UserSettingsRepository::save(&**store, &UserSettings::new(user_id.clone()))
        .await
        .unwrap();
    store
        .set_fields(
            &user_id,
            vec![ProfileField {
                name: "Site".into(),
                value: "https://example.com".into(),
            }],
            &[],
        )
        .await
        .unwrap();
    WebhookTokenRepository::save(
        &**store,
        &WebhookToken::new(
            user_id.clone(),
            "hash".into(),
            WatchEventSource::Jellyfin,
            None,
        ),
    )
    .await
    .unwrap();
    WatchEventCommand::save(
        &**store,
        &WatchEvent::new(
            user_id.clone(),
            "Heat".into(),
            Some(1995),
            None,
            WatchEventSource::Jellyfin,
            test_watched_at(),
            None,
        ),
        &[],
    )
    .await
    .unwrap();
    store
        .follow(
            &user_id,
            &FollowTarget::Identity(SocialIdentity::Remote {
                actor_url: REMOTE_ACTOR.into(),
            }),
            &[],
        )
        .await
        .unwrap();
    store
        .follow(
            other.id(),
            &FollowTarget::Identity(SocialIdentity::Local(user_id.clone())),
            &[],
        )
        .await
        .unwrap();
    store
        .block(
            &user_id,
            &SocialIdentity::Remote {
                actor_url: REMOTE_ACTOR.into(),
            },
            &[],
        )
        .await
        .unwrap();

    store
        .save_review(&test_review(movie.id(), &user_id, 4), &[])
        .await
        .unwrap();
    store
        .create(&RefreshSession {
            id: Uuid::new_v4(),
            user_id: user_id.clone(),
            token: "refresh-token".into(),
            expires_at: Utc::now() + chrono::Duration::days(1),
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    (user_id, movie)
}

#[tokio::test]
async fn deleting_a_user_removes_watchlist_goals_settings_profile_fields_tokens_events_follows_and_blocks()
 {
    let store = InMemoryStore::new();
    let (user_id, _) = seed_user_with_dependents(&store).await;
    for table in USER_CASCADE_TABLES {
        assert!(store.rows_in(table) > 0, "{table} should be seeded");
    }

    store.delete_user(&user_id);

    assert!(store.find_by_id(&user_id).await.unwrap().is_none());
    for table in USER_CASCADE_TABLES {
        assert_eq!(store.rows_in(table), 0, "{table} should cascade");
    }
}

#[tokio::test]
async fn deleting_a_user_keeps_reviews_and_refresh_sessions_because_the_schema_has_no_foreign_key()
{
    let store = InMemoryStore::new();
    let (user_id, _) = seed_user_with_dependents(&store).await;

    store.delete_user(&user_id);

    assert_eq!(store.review_count(), 1);
    assert_eq!(store.refresh_session_count(), 1);
}

#[tokio::test]
async fn deleting_a_user_leaves_other_users_rows_alone() {
    let store = InMemoryStore::new();
    let (user_id, movie) = seed_user_with_dependents(&store).await;
    let bystander = test_user("carol");
    UserRepository::save(&*store, &bystander).await.unwrap();
    store
        .add(
            &WatchlistEntry::new(bystander.id().clone(), movie.id().clone()),
            &[],
        )
        .await
        .unwrap();

    store.delete_user(&user_id);

    assert_eq!(store.watchlist_count(), 1);
    assert!(store.find_by_id(bystander.id()).await.unwrap().is_some());
}

async fn seed_movie_with_dependents(
    store: &Arc<InMemoryStore>,
    with_review: bool,
) -> (Movie, UserId) {
    let user = test_user("alice");
    UserRepository::save(&**store, &user).await.unwrap();
    let movie = test_movie("Heat");
    store.upsert_movie(&movie, &[]).await.unwrap();
    store
        .add(
            &WatchlistEntry::new(user.id().clone(), movie.id().clone()),
            &[],
        )
        .await
        .unwrap();
    store.upsert(&movie_profile(movie.id())).await.unwrap();
    WatchEventCommand::save(
        &**store,
        &WatchEvent::new(
            user.id().clone(),
            "Heat".into(),
            Some(1995),
            None,
            WatchEventSource::Plex,
            test_watched_at(),
            Some(movie.id().clone()),
        ),
        &[],
    )
    .await
    .unwrap();
    if with_review {
        store
            .save_review(&test_review(movie.id(), user.id(), 5), &[])
            .await
            .unwrap();
    }
    (movie, user.id().clone())
}

#[tokio::test]
async fn deleting_a_movie_removes_watchlist_entries_and_enrichment_profile() {
    let store = InMemoryStore::new();
    let (movie, _) = seed_movie_with_dependents(&store, false).await;
    for table in MOVIE_CASCADE_TABLES {
        assert!(store.rows_in(table) > 0, "{table} should be seeded");
    }

    store.delete_movie(movie.id(), &[]).await.unwrap();

    assert_eq!(store.movie_count(), 0);
    for table in MOVIE_CASCADE_TABLES {
        assert_eq!(store.rows_in(table), 0, "{table} should cascade");
    }
}

#[tokio::test]
async fn deleting_a_movie_detaches_watch_events_instead_of_removing_them() {
    let store = InMemoryStore::new();
    let (movie, user_id) = seed_movie_with_dependents(&store, false).await;

    store.delete_movie(movie.id(), &[]).await.unwrap();

    let pending = store.list_pending(&user_id).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].movie_id().is_none());
}

#[tokio::test]
async fn deleting_a_movie_that_has_reviews_is_refused_because_reviews_do_not_cascade() {
    let store = InMemoryStore::new();
    let (movie, _) = seed_movie_with_dependents(&store, true).await;

    let result = store.delete_movie(movie.id(), &[]).await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(_))));
    assert_eq!(store.movie_count(), 1);
    assert_eq!(store.review_count(), 1);
    assert_eq!(store.watchlist_count(), 1);
}

#[tokio::test]
async fn saving_a_user_with_a_taken_username_is_a_validation_error() {
    let store = InMemoryStore::new();
    UserRepository::save(&*store, &test_user("alice"))
        .await
        .unwrap();
    let clash = domain::models::User::new(
        domain::value_objects::Email::new("other@example.com".into()).unwrap(),
        domain::value_objects::Username::new("alice".into()).unwrap(),
        domain::value_objects::PasswordHash::new("hashed".into()).unwrap(),
        domain::models::UserRole::Standard,
    );

    let result = UserRepository::save(&*store, &clash).await;

    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn saving_a_user_with_a_taken_email_is_a_validation_error() {
    let store = InMemoryStore::new();
    UserRepository::save(&*store, &test_user("alice"))
        .await
        .unwrap();
    let clash = domain::models::User::new(
        domain::value_objects::Email::new("alice@example.com".into()).unwrap(),
        domain::value_objects::Username::new("alice2".into()).unwrap(),
        domain::value_objects::PasswordHash::new("hashed".into()).unwrap(),
        domain::models::UserRole::Standard,
    );

    let result = UserRepository::save(&*store, &clash).await;

    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[tokio::test]
async fn saving_a_second_goal_for_the_same_user_and_year_is_an_infrastructure_error() {
    let store = InMemoryStore::new();
    let user_id = UserId::generate();
    GoalCommand::save(&*store, &test_goal(&user_id, 2024, 10), &[])
        .await
        .unwrap();

    let result = GoalCommand::save(&*store, &test_goal(&user_id, 2024, 20), &[]).await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(_))));
    assert_eq!(store.goal_count(), 1);
}

#[tokio::test]
async fn adding_the_same_movie_to_a_watchlist_twice_keeps_one_entry() {
    let store = InMemoryStore::new();
    let user_id = UserId::generate();
    let movie_id = MovieId::generate();

    store
        .add(&WatchlistEntry::new(user_id.clone(), movie_id.clone()), &[])
        .await
        .unwrap();
    store
        .add(&WatchlistEntry::new(user_id, movie_id), &[])
        .await
        .unwrap();

    assert_eq!(store.watchlist_count(), 1);
}

#[tokio::test]
async fn upserting_a_different_movie_with_a_taken_external_id_is_an_infrastructure_error() {
    let store = InMemoryStore::new();
    let external = ExternalMetadataId::new("tt0113277".into()).unwrap();
    store
        .upsert_movie(
            &test_movie_with_external_id("Heat", 1995, external.clone()),
            &[],
        )
        .await
        .unwrap();

    let result = store
        .upsert_movie(&test_movie_with_external_id("Heat 2", 2030, external), &[])
        .await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(_))));
    assert_eq!(store.movie_count(), 1);
}

#[tokio::test]
async fn creating_a_refresh_session_with_a_taken_token_is_an_infrastructure_error() {
    let store = InMemoryStore::new();
    let session = |token: &str| RefreshSession {
        id: Uuid::new_v4(),
        user_id: UserId::generate(),
        token: token.into(),
        expires_at: Utc::now() + chrono::Duration::days(1),
        created_at: Utc::now(),
    };
    store.create(&session("same")).await.unwrap();

    let result = store.create(&session("same")).await;

    assert!(matches!(result, Err(DomainError::InfrastructureError(_))));
}

fn movie_discovered(movie: &Movie) -> domain::events::DomainEvent {
    domain::events::DomainEvent::MovieDiscovered {
        movie_id: movie.id().clone(),
        external_metadata_id: ExternalMetadataId::new("tmdb:1".into()).unwrap(),
    }
}

fn movie_deleted(movie: &Movie) -> domain::events::DomainEvent {
    domain::events::DomainEvent::MovieDeleted {
        movie_id: movie.id().clone(),
        poster_path: None,
    }
}

#[tokio::test]
async fn upserting_a_movie_records_its_events() {
    let store = InMemoryStore::new();
    let movie = test_movie("Heat");

    store
        .upsert_movie(&movie, &[movie_discovered(&movie)])
        .await
        .unwrap();

    assert_eq!(store.movie_count(), 1);
    assert_eq!(store.recorded_events().len(), 1);
}

#[tokio::test]
async fn a_refused_movie_upsert_records_no_events() {
    let store = InMemoryStore::new();
    let external = ExternalMetadataId::new("tt0113277".into()).unwrap();
    let first = test_movie_with_external_id("Heat", 1995, external.clone());
    store.upsert_movie(&first, &[]).await.unwrap();
    let clash = test_movie_with_external_id("Heat 2", 2030, external);

    let result = store
        .upsert_movie(&clash, &[movie_discovered(&clash)])
        .await;

    assert!(result.is_err());
    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn deleting_a_movie_records_its_events() {
    let store = InMemoryStore::new();
    let movie = test_movie("Heat");
    store.upsert_movie(&movie, &[]).await.unwrap();

    store
        .delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await
        .unwrap();

    assert_eq!(store.recorded_events().len(), 1);
}

#[tokio::test]
async fn deleting_an_absent_movie_records_no_events() {
    let store = InMemoryStore::new();
    let movie = test_movie("Heat");

    store
        .delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await
        .unwrap();

    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn a_refused_movie_delete_records_no_events() {
    let store = InMemoryStore::new();
    let (movie, _) = seed_movie_with_dependents(&store, true).await;

    let result = store
        .delete_movie(movie.id(), &[movie_deleted(&movie)])
        .await;

    assert!(result.is_err());
    assert!(store.recorded_events().is_empty());
}

#[tokio::test]
async fn saving_a_watch_event_records_its_events() {
    let store = InMemoryStore::new();
    let user_id = UserId::generate();
    let event = WatchEvent::new(
        user_id.clone(),
        "Heat".into(),
        Some(1995),
        None,
        WatchEventSource::Jellyfin,
        test_watched_at(),
        None,
    );

    WatchEventCommand::save(
        &*store,
        &event,
        &[domain::events::DomainEvent::WatchEventIngested {
            user_id,
            title: "Heat".into(),
            source: "jellyfin".into(),
        }],
    )
    .await
    .unwrap();

    assert_eq!(store.rows_in("watch_events"), 1);
    assert_eq!(store.recorded_events().len(), 1);
}

#[tokio::test]
async fn deleting_the_last_review_removes_the_movie_and_records_both_events() {
    let store = InMemoryStore::new();
    let (movie, user_id) = seed_movie_with_dependents(&store, true).await;
    let review = store
        .get_all_reviews_for_user(&user_id)
        .await
        .unwrap()
        .remove(0);

    store
        .delete_review_and_orphaned_movie(
            review.id(),
            movie.id(),
            &[domain::events::DomainEvent::ReviewDeleted {
                review_id: review.id().clone(),
                user_id: user_id.clone(),
            }],
            &[movie_deleted(&movie)],
        )
        .await
        .unwrap();

    assert_eq!(store.review_count(), 0);
    assert_eq!(store.movie_count(), 0);
    assert_eq!(store.recorded_events().len(), 2);
}

#[tokio::test]
async fn deleting_a_review_while_another_remains_keeps_the_movie_and_records_one_event() {
    let store = InMemoryStore::new();
    let (movie, user_id) = seed_movie_with_dependents(&store, true).await;
    let review = store
        .get_all_reviews_for_user(&user_id)
        .await
        .unwrap()
        .remove(0);
    let other = test_review(movie.id(), &UserId::generate(), 3);
    store.save_review(&other, &[]).await.unwrap();

    store
        .delete_review_and_orphaned_movie(
            review.id(),
            movie.id(),
            &[domain::events::DomainEvent::ReviewDeleted {
                review_id: review.id().clone(),
                user_id: user_id.clone(),
            }],
            &[movie_deleted(&movie)],
        )
        .await
        .unwrap();

    assert_eq!(store.review_count(), 1);
    assert_eq!(store.movie_count(), 1);
    assert_eq!(store.recorded_events().len(), 1);
}

#[tokio::test]
async fn fake_mark_deleted_mirrors_the_sqlite_tombstone() {
    let store = InMemoryStore::new();
    let user = test_user("ghost");
    let other = test_user("other");
    UserRepository::save(&*store, &user).await.unwrap();
    UserRepository::save(&*store, &other).await.unwrap();
    RefreshSessionRepository::create(
        &*store,
        &RefreshSession {
            id: Uuid::new_v4(),
            user_id: user.id().clone(),
            token: "t".into(),
            expires_at: Utc::now(),
            created_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    let event = domain::events::DomainEvent::UserDeleted {
        user_id: user.id().clone(),
    };

    store
        .mark_deleted(user.id(), std::slice::from_ref(&event))
        .await
        .unwrap();

    assert!(store.find_by_id(user.id()).await.unwrap().is_none());
    assert!(store.find_by_email(user.email()).await.unwrap().is_none());
    assert!(
        store
            .find_by_username(user.username())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(store.list_with_stats().await.unwrap().len(), 1);
    assert!(
        store
            .find_tombstoned_by_id(user.id())
            .await
            .unwrap()
            .is_some()
    );
    assert!(store.is_username_retired(user.username()).await.unwrap());
    let upper =
        domain::value_objects::Username::new(user.username().value().to_uppercase()).unwrap();
    assert!(store.is_username_retired(&upper).await.unwrap());
    assert_eq!(store.refresh_session_count(), 0);
    assert_eq!(store.recorded_events().len(), 1);
}

#[tokio::test]
async fn fake_mark_deleted_refuses_unknown_and_already_deleted_users() {
    let store = InMemoryStore::new();
    let user = test_user("ghost");
    UserRepository::save(&*store, &user).await.unwrap();
    let event = domain::events::DomainEvent::UserDeleted {
        user_id: user.id().clone(),
    };

    let unknown = store
        .mark_deleted(&UserId::generate(), std::slice::from_ref(&event))
        .await;
    assert!(matches!(unknown, Err(DomainError::NotFound(_))));
    assert!(store.recorded_events().is_empty());

    store
        .mark_deleted(user.id(), std::slice::from_ref(&event))
        .await
        .unwrap();
    let again = store
        .mark_deleted(user.id(), std::slice::from_ref(&event))
        .await;
    assert!(matches!(again, Err(DomainError::NotFound(_))));
    assert_eq!(
        store.recorded_events().len(),
        1,
        "second call records nothing"
    );
}
