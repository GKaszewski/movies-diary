use crate::support::generate_webhook_token_command;
use std::sync::Arc;

use domain::models::WatchEventSource;
use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryStore;
use uuid::Uuid;

use application::integrations::deps::{GenerateWebhookTokenDeps, GetWebhookTokensDeps};
use application::integrations::{
    commands::GenerateWebhookTokenCommand, generate_token, get_tokens,
    queries::GetWebhookTokensQuery,
};

fn generate_deps(tokens: &Arc<dyn WebhookTokenRepository>) -> GenerateWebhookTokenDeps {
    GenerateWebhookTokenDeps {
        webhook_token: Arc::clone(tokens),
    }
}

fn get_deps(tokens: &Arc<dyn WebhookTokenRepository>) -> GetWebhookTokensDeps {
    GetWebhookTokensDeps {
        webhook_token: Arc::clone(tokens),
    }
}

#[tokio::test]
async fn returns_empty_when_no_tokens() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryStore::new();

    let result = get_tokens::execute(
        &get_deps(&tokens),
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
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryStore::new();

    let user_id = Uuid::new_v4();

    generate_token::execute(
        &generate_deps(&tokens),
        generate_webhook_token_command(user_id),
    )
    .await
    .unwrap();

    generate_token::execute(
        &generate_deps(&tokens),
        GenerateWebhookTokenCommand {
            provider: WatchEventSource::Plex,
            label: Some("living room".into()),
            ..generate_webhook_token_command(user_id)
        },
    )
    .await
    .unwrap();

    let result = get_tokens::execute(&get_deps(&tokens), GetWebhookTokensQuery { user_id })
        .await
        .unwrap();

    assert_eq!(result.len(), 2);
}
