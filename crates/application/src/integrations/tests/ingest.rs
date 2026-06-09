use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::commands::GenerateWebhookTokenCommand;
use crate::integrations::{commands::IngestWatchEventCommand, generate_token, ingest};
use crate::test_helpers::TestContextBuilder;

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
    let tokens = InMemoryWebhookTokenRepository::new();
    let ctx = TestContextBuilder::new()
        .with_webhook_tokens(Arc::clone(&tokens) as _)
        .build();

    let user_id = Uuid::new_v4();
    let generated = generate_token::execute(
        &ctx,
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Jellyfin,
            label: None,
        },
    )
    .await
    .unwrap();

    let result = ingest::execute(
        &ctx,
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
    let ctx = TestContextBuilder::new().build();

    let result = ingest::execute(
        &ctx,
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
