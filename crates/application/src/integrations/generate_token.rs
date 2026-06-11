use std::sync::Arc;

use domain::{
    errors::DomainError, models::WebhookToken, ports::WebhookTokenRepository, value_objects::UserId,
};
use sha2::{Digest, Sha256};

use crate::integrations::commands::GenerateWebhookTokenCommand;

pub struct GeneratedWebhookToken {
    pub token_plaintext: String,
    pub token: WebhookToken,
}

pub async fn execute(
    webhook_token: Arc<dyn WebhookTokenRepository>,
    cmd: GenerateWebhookTokenCommand,
) -> Result<GeneratedWebhookToken, DomainError> {
    let plaintext = generate_random_token();
    let hash = hash_token(&plaintext);

    let user_id = UserId::from_uuid(cmd.user_id);
    let token = WebhookToken::new(user_id, hash, cmd.provider, cmd.label);

    webhook_token.save(&token).await?;

    Ok(GeneratedWebhookToken {
        token_plaintext: plaintext,
        token,
    })
}

fn generate_random_token() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

pub fn hash_token(plaintext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
#[path = "tests/generate_token.rs"]
mod tests;
