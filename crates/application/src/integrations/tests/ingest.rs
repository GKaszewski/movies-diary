use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::ports::{EventPublisher, WatchEventRepository, WebhookTokenRepository};
use domain::testing::{
    InMemoryWatchEventRepository, InMemoryWebhookTokenRepository, NoopEventPublisher,
};
use uuid::Uuid;

use crate::integrations::commands::{GenerateWebhookTokenCommand, IngestWatchEventCommand};
use crate::integrations::deps::IngestWatchEventDeps;
use crate::integrations::{generate_token, ingest};

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
async fn ingests_watch_event() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();
    let watch_events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();
    let event_publisher: Arc<dyn EventPublisher> = NoopEventPublisher::new();

    let user_id = Uuid::new_v4();
    let generated = generate_token::execute(
        Arc::clone(&tokens),
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Jellyfin,
            label: None,
        },
    )
    .await
    .unwrap();

    let deps = IngestWatchEventDeps {
        webhook_token: Arc::clone(&tokens),
        watch_event: Arc::clone(&watch_events),
        event_publisher: Arc::clone(&event_publisher),
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
}

#[tokio::test]
async fn rejects_invalid_token() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();
    let watch_events: Arc<dyn WatchEventRepository> = InMemoryWatchEventRepository::new();
    let event_publisher: Arc<dyn EventPublisher> = NoopEventPublisher::new();

    let deps = IngestWatchEventDeps {
        webhook_token: Arc::clone(&tokens),
        watch_event: Arc::clone(&watch_events),
        event_publisher: Arc::clone(&event_publisher),
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
