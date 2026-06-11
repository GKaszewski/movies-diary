use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::{
    commands::GenerateWebhookTokenCommand, generate_token, get_tokens,
    queries::GetWebhookTokensQuery,
};

#[tokio::test]
async fn returns_empty_when_no_tokens() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();

    let result = get_tokens::execute(
        Arc::clone(&tokens),
        GetWebhookTokensQuery {
            user_id: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn returns_tokens_after_generate() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();

    let user_id = Uuid::new_v4();

    generate_token::execute(
        Arc::clone(&tokens),
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Jellyfin,
            label: None,
        },
    )
    .await
    .unwrap();

    generate_token::execute(
        Arc::clone(&tokens),
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Plex,
            label: Some("living room".into()),
        },
    )
    .await
    .unwrap();

    let result = get_tokens::execute(Arc::clone(&tokens), GetWebhookTokensQuery { user_id })
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
}
