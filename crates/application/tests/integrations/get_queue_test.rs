use std::sync::Arc;

use chrono::Utc;
use domain::models::{WatchEvent, WatchEventSource};
use domain::ports::WatchEventCommand;
use domain::testing::InMemoryStore;
use domain::value_objects::UserId;
use uuid::Uuid;

use application::integrations::deps::GetWatchQueueDeps;
use application::integrations::{get_queue, queries::GetWatchQueueQuery};

fn deps(events: &Arc<InMemoryStore>) -> GetWatchQueueDeps {
    GetWatchQueueDeps {
        watch_event_query: Arc::clone(events) as _,
    }
}

#[tokio::test]
async fn returns_empty_when_no_events() {
    let events = InMemoryStore::new();

    let result = get_queue::execute(
        &deps(&events),
        GetWatchQueueQuery {
            user_id: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn returns_pending_events() {
    let events = InMemoryStore::new();

    let user_id = Uuid::new_v4();
    let event = WatchEvent::new(
        UserId::from_uuid(user_id),
        "Blade Runner 2049".into(),
        Some(2017),
        None,
        WatchEventSource::Jellyfin,
        Utc::now().naive_utc(),
        None,
    );
    events.save(&event, &[]).await.unwrap();

    let result = get_queue::execute(&deps(&events), GetWatchQueueQuery { user_id })
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
}
