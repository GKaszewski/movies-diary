use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryWebhookTokenRepository;
use uuid::Uuid;

use crate::integrations::{
    commands::{GenerateWebhookTokenCommand, RevokeWebhookTokenCommand},
    generate_token, get_tokens,
    queries::GetWebhookTokensQuery,
    revoke_token,
};

#[tokio::test]
async fn revokes_existing_token() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryWebhookTokenRepository::new();

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

    let token_id = generated.token.id().value();

    revoke_token::execute(
        Arc::clone(&tokens),
        RevokeWebhookTokenCommand { user_id, token_id },
    )
    .await
    .unwrap();

    let remaining = get_tokens::execute(Arc::clone(&tokens), GetWebhookTokensQuery { user_id })
        .await
        .unwrap();

    assert!(remaining.is_empty());
}
