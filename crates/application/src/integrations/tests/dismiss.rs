use std::sync::Arc;

use domain::models::{WatchEvent, WatchEventSource};
use domain::ports::WatchEventRepository;
use domain::testing::InMemoryWatchEventRepository;
use domain::value_objects::UserId;
use uuid::Uuid;

use crate::integrations::{commands::DismissWatchEventsCommand, dismiss};

#[tokio::test]
async fn dismisses_empty_list_returns_zero() {
    let events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();

    let result = dismiss::execute(
        Arc::clone(&events),
        DismissWatchEventsCommand {
            user_id: Uuid::new_v4(),
            event_ids: vec![],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 0);
}

#[tokio::test]
async fn fails_when_event_not_found() {
    let events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();

    let result = dismiss::execute(
        Arc::clone(&events),
        DismissWatchEventsCommand {
            user_id: Uuid::new_v4(),
            event_ids: vec![Uuid::new_v4()],
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn dismisses_existing_events() {
    let watch_events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();
    let uid = Uuid::new_v4();
    let user_id = UserId::from_uuid(uid);

    let e1 = WatchEvent::new(
        user_id.clone(),
        "Movie A".into(),
        Some(2024),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let e2 = WatchEvent::new(
        user_id,
        "Movie B".into(),
        Some(2023),
        None,
        WatchEventSource::Jellyfin,
        chrono::Utc::now().naive_utc(),
        None,
    );
    let id1 = e1.id().value();
    let id2 = e2.id().value();
    watch_events.save(&e1).await.unwrap();
    watch_events.save(&e2).await.unwrap();

    let result = dismiss::execute(
        Arc::clone(&watch_events),
        DismissWatchEventsCommand {
            user_id: uid,
            event_ids: vec![id1, id2],
        },
    )
    .await
    .unwrap();

    assert_eq!(result, 2);
}
