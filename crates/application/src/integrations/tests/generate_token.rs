use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::{commands::GenerateWebhookTokenCommand, generate_token};

#[tokio::test]
async fn generates_token_and_saves() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();

    let user_id = Uuid::new_v4();
    let result = generate_token::execute(
        Arc::clone(&tokens),
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Jellyfin,
            label: None,
        },
    )
    .await
    .unwrap();

    assert!(!result.token_plaintext.is_empty());

    let saved = tokens
        .list_by_user(&domain::value_objects::UserId::from_uuid(user_id))
        .await
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id().value(), result.token.id().value());
}
