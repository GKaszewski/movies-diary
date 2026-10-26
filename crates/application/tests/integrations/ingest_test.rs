use crate::support::generate_webhook_token_command;
use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::testing::{InMemoryStore, RefusingWatchEventCommand};
use uuid::Uuid;

use application::integrations::commands::IngestWatchEventCommand;
use application::integrations::deps::{GenerateWebhookTokenDeps, IngestWatchEventDeps};
use application::integrations::{generate_token, ingest};

struct FakeParser;

impl domain::ports::MediaServerParser for FakeParser {
    fn parse_playback_event(
        &self,
        _: &[u8],
    ) -> Result<Option<domain::models::ParsedPlaybackEvent>, domain::errors::DomainError> {
        Ok(Some(domain::models::ParsedPlaybackEvent {
            title: "Test".into(),
            year: Some(2024),
            tmdb_id: None,
            imdb_id: None,
        }))
    }
}

#[tokio::test]
async fn ingesting_saves_the_watch_event_and_records_its_event_together() {
    let tokens = InMemoryStore::new();
    let watch_events = Arc::clone(&tokens);

    let user_id = Uuid::new_v4();
    let generated = generate_token::execute(
        &GenerateWebhookTokenDeps {
            webhook_token: Arc::clone(&tokens) as _,
        },
        generate_webhook_token_command(user_id),
    )
    .await
    .unwrap();

    let deps = IngestWatchEventDeps {
        webhook_token: Arc::clone(&tokens) as _,
        watch_event_command: Arc::clone(&watch_events) as _,
        watch_event_query: Arc::clone(&watch_events) as _,
    };

    let result = ingest::execute(
        &deps,
        IngestWatchEventCommand {
            token: generated.token_plaintext,
            raw_payload: vec![],
            source: WatchEventSource::Jellyfin,
        },
        &FakeParser,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(watch_events.rows_in("watch_events"), 1);
    let recorded = tokens.recorded_events();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        &recorded[0],
        domain::events::DomainEvent::WatchEventIngested { title, .. } if title == "Test"
    ));
}

#[tokio::test]
async fn a_refused_save_propagates_and_records_nothing() {
    let tokens = InMemoryStore::new();
    let user_id = Uuid::new_v4();
    let generated = generate_token::execute(
        &GenerateWebhookTokenDeps {
            webhook_token: Arc::clone(&tokens) as _,
        },
        generate_webhook_token_command(user_id),
    )
    .await
    .unwrap();
    let deps = IngestWatchEventDeps {
        webhook_token: Arc::clone(&tokens) as _,
        watch_event_command: Arc::new(RefusingWatchEventCommand),
        watch_event_query: Arc::clone(&tokens) as _,
    };

    let result = ingest::execute(
        &deps,
        IngestWatchEventCommand {
            token: generated.token_plaintext,
            raw_payload: vec![],
            source: WatchEventSource::Jellyfin,
        },
        &FakeParser,
    )
    .await;

    assert!(result.is_err());
    assert_eq!(tokens.rows_in("watch_events"), 0);
    assert!(tokens.recorded_events().is_empty());
}

#[tokio::test]
async fn rejects_invalid_token() {
    let tokens = InMemoryStore::new();
    let watch_events = Arc::clone(&tokens);

    let deps = IngestWatchEventDeps {
        webhook_token: Arc::clone(&tokens) as _,
        watch_event_command: Arc::clone(&watch_events) as _,
        watch_event_query: Arc::clone(&watch_events) as _,
    };

    let result = ingest::execute(
        &deps,
        IngestWatchEventCommand {
            token: "bad-token".into(),
            raw_payload: vec![],
            source: WatchEventSource::Jellyfin,
        },
        &FakeParser,
    )
    .await;

    assert!(result.is_err());
}
