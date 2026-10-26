use crate::support::generate_webhook_token_command;
use std::sync::Arc;

use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryStore;
use uuid::Uuid;

use application::integrations::deps::{
    GenerateWebhookTokenDeps, GetWebhookTokensDeps, RevokeWebhookTokenDeps,
};
use application::integrations::{
    commands::RevokeWebhookTokenCommand, generate_token, get_tokens,
    queries::GetWebhookTokensQuery, revoke_token,
};

#[tokio::test]
async fn revokes_existing_token() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryStore::new();

    let user_id = Uuid::new_v4();

    let generated = generate_token::execute(
        &GenerateWebhookTokenDeps {
            webhook_token: Arc::clone(&tokens),
        },
        generate_webhook_token_command(user_id),
    )
    .await
    .unwrap();

    let token_id = generated.token.id().value();

    revoke_token::execute(
        &RevokeWebhookTokenDeps {
            webhook_token: Arc::clone(&tokens),
        },
        RevokeWebhookTokenCommand { user_id, token_id },
    )
    .await
    .unwrap();

    let remaining = get_tokens::execute(
        &GetWebhookTokensDeps {
            webhook_token: Arc::clone(&tokens),
        },
        GetWebhookTokensQuery { user_id },
    )
    .await
    .unwrap();

    assert!(remaining.is_empty());
}
