use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::{commands::GenerateWebhookTokenCommand, generate_token};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn generates_token_and_saves() {
    let tokens = InMemoryWebhookTokenRepository::new();
    let ctx = TestContextBuilder::new()
        .with_webhook_tokens(Arc::clone(&tokens) as _)
        .build();

    let user_id = Uuid::new_v4();
    let result = generate_token::execute(
        &ctx,
        GenerateWebhookTokenCommand {
            user_id,
            provider: WatchEventSource::Jellyfin,
            label: None,
        },
    )
    .await
    .unwrap();

    assert!(!result.token_plaintext.is_empty());

    let saved = ctx
        .repos
        .webhook_token
        .list_by_user(&domain::value_objects::UserId::from_uuid(user_id))
        .await
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id().value(), result.token.id().value());
}
