use crate::support::dismiss_watch_events_command;
use std::sync::Arc;

use domain::models::{WatchEvent, WatchEventSource};
use domain::ports::WatchEventCommand;
use domain::testing::InMemoryStore;
use domain::value_objects::UserId;
use uuid::Uuid;

use application::integrations::deps::DismissWatchEventsDeps;
use application::integrations::dismiss;

fn deps(watch_events: &Arc<InMemoryStore>) -> DismissWatchEventsDeps {
    DismissWatchEventsDeps {
        watch_event_command: Arc::clone(watch_events) as _,
        watch_event_query: Arc::clone(watch_events) as _,
    }
}

#[tokio::test]
async fn dismisses_empty_list_returns_zero() {
    let events = InMemoryStore::new();

    let result = dismiss::execute(
        &deps(&events),
        dismiss_watch_events_command(Uuid::new_v4(), vec![]),
    )
    .await
    .unwrap();

    assert_eq!(result, 0);
}

#[tokio::test]
async fn fails_when_event_not_found() {
    let events = InMemoryStore::new();

    let result = dismiss::execute(
        &deps(&events),
        dismiss_watch_events_command(Uuid::new_v4(), vec![Uuid::new_v4()]),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn dismisses_existing_events() {
    let watch_events = InMemoryStore::new();
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
    watch_events.save(&e1, &[]).await.unwrap();
    watch_events.save(&e2, &[]).await.unwrap();

    let result = dismiss::execute(
        &deps(&watch_events),
        dismiss_watch_events_command(uid, vec![id1, id2]),
    )
    .await
    .unwrap();

    assert_eq!(result, 2);
}
