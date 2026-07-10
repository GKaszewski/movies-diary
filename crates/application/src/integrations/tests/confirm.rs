use std::sync::Arc;

use domain::models::{WatchEvent, WatchEventSource};
use domain::ports::{MovieCommand, WatchEventCommand};
use domain::testing::{InMemoryWatchEventRepository, NoopEventPublisher};
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::integrations::commands::{ConfirmWatchEventsCommand, WatchEventConfirmation};
use crate::integrations::confirm;
use crate::test_helpers::NoopReviewLogger;

fn noop_logger() -> Arc<dyn crate::ports::ReviewLogger> {
    Arc::new(NoopReviewLogger)
}

#[tokio::test]
async fn confirms_watch_event_via_review_logger() {
    let watch_events = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(uid),
        "Test Movie".into(),
        Some(2024),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 4,
                comment: None,
            }],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 1);
}

#[tokio::test]
async fn empty_confirmations_returns_zero() {
    let watch_events = InMemoryWatchEventRepository::new();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: Uuid::new_v4(),
            confirmations: vec![],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 0);
}

#[tokio::test]
async fn confirms_event_with_external_metadata_id_and_no_movie_id() {
    let watch_events = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(uid),
        "External Movie".into(),
        Some(2023),
        Some("tt1234567".into()),
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 3,
                comment: Some("Great film".into()),
            }],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 1);
}

#[tokio::test]
async fn rejects_other_users_event() {
    let watch_events = InMemoryWatchEventRepository::new();
    let owner = Uuid::new_v4();
    let intruder = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(owner),
        "Movie".into(),
        None,
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: intruder,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 3,
                comment: None,
            }],
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn fails_when_event_not_found() {
    let watch_events = InMemoryWatchEventRepository::new();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: Uuid::new_v4(),
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: Uuid::new_v4(),
                rating: 4,
                comment: None,
            }],
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn confirms_event_with_movie_id() {
    let watch_events = InMemoryWatchEventRepository::new();
    let events = NoopEventPublisher::new();
    let uid = Uuid::new_v4();
    let movie_uuid = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(uid),
        "Movie With Id".into(),
        Some(2024),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        Some(domain::value_objects::MovieId::from_uuid(movie_uuid)),
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    // Also seed movie repo so review_logger can find it
    let movies = domain::testing::InMemoryMovieRepository::new();
    let movie = domain::models::Movie::from_persistence(
        domain::value_objects::MovieId::from_uuid(movie_uuid),
        None,
        domain::value_objects::MovieTitle::new("Movie With Id".into()).unwrap(),
        domain::value_objects::ReleaseYear::new(2024).unwrap(),
        None,
        None,
    );
    movies.upsert_movie(&movie).await.unwrap();

    // Build a real review logger
    let reviews = domain::testing::InMemoryReviewRepository::new();
    let watchlist = domain::testing::InMemoryWatchlistRepository::new();
    let review_logger: Arc<dyn crate::ports::ReviewLogger> =
        Arc::new(crate::diary::review_logger::DefaultReviewLogger::new(
            Arc::clone(&movies) as _,
            Arc::clone(&movies) as _,
            Arc::clone(&reviews) as _,
            Arc::clone(&watchlist) as _,
            Arc::new(domain::testing::FakeMetadataClient) as _,
            Arc::clone(&events) as _,
        ));

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        review_logger,
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 4,
                comment: None,
            }],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 1);
}

#[tokio::test]
async fn confirms_event_without_movie_id_and_without_external_metadata_id() {
    let watch_events = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(uid),
        "Title Only Movie".into(),
        Some(2022),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 5,
                comment: Some("Amazing".into()),
            }],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 1);
}

#[tokio::test]
async fn confirms_multiple_events() {
    let watch_events = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();

    let event1 = WatchEvent::new(
        UserId::from_uuid(uid),
        "Movie One".into(),
        Some(2020),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let id1 = event1.id().value();

    let event2 = WatchEvent::new(
        UserId::from_uuid(uid),
        "Movie Two".into(),
        Some(2021),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let id2 = event2.id().value();

    watch_events.save(&event1).await.unwrap();
    watch_events.save(&event2).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![
                WatchEventConfirmation {
                    watch_event_id: id1,
                    rating: 3,
                    comment: None,
                },
                WatchEventConfirmation {
                    watch_event_id: id2,
                    rating: 4,
                    comment: None,
                },
            ],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 2);
}

#[tokio::test]
async fn confirms_event_without_year() {
    let watch_events = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();

    let event = WatchEvent::new(
        UserId::from_uuid(uid),
        "No Year Movie".into(),
        None,
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let event_id = event.id().value();
    watch_events.save(&event).await.unwrap();

    let result = confirm::execute(
        Arc::clone(&watch_events) as _,
        Arc::clone(&watch_events) as _,
        noop_logger(),
        ConfirmWatchEventsCommand {
            user_id: uid,
            confirmations: vec![WatchEventConfirmation {
                watch_event_id: event_id,
                rating: 3,
                comment: None,
            }],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 1);
}
