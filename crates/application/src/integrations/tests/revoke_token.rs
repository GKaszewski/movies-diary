use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::{
    commands::{GenerateWebhookTokenCommand, RevokeWebhookTokenCommand},
    generate_token, get_tokens,
    queries::GetWebhookTokensQuery,
    revoke_token,
};
use crate::test_helpers::TestContextBuilder;

#[tokio::test]
async fn revokes_existing_token() {
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

    let token_id = generated.token.id().value();

    revoke_token::execute(&ctx, RevokeWebhookTokenCommand { user_id, token_id })
        .await
        .unwrap();

    let remaining = get_tokens::execute(&ctx, GetWebhookTokensQuery { user_id })
        .await
        .unwrap();

    assert!(remaining.is_empty());
}
