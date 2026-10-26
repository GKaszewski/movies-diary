use crate::support::generate_webhook_token_command;
use std::sync::Arc;

use domain::ports::WebhookTokenRepository;
use domain::testing::InMemoryStore;
use uuid::Uuid;

use application::integrations::deps::GenerateWebhookTokenDeps;
use application::integrations::generate_token;

#[tokio::test]
async fn generates_token_and_saves() {
    let tokens: Arc<dyn WebhookTokenRepository> = InMemoryStore::new();

    let user_id = Uuid::new_v4();
    let result = generate_token::execute(
        &GenerateWebhookTokenDeps {
            webhook_token: Arc::clone(&tokens),
        },
        generate_webhook_token_command(user_id),
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
