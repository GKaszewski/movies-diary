use std::sync::Arc;

use domain::{
    errors::DomainError,
    ports::WebhookTokenRepository,
    value_objects::{UserId, WebhookTokenId},
};

use crate::integrations::commands::RevokeWebhookTokenCommand;

pub async fn execute(
    webhook_token: Arc<dyn WebhookTokenRepository>,
    cmd: RevokeWebhookTokenCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let token_id = WebhookTokenId::from_uuid(cmd.token_id);
    webhook_token.delete(&token_id, &user_id).await
}

#[cfg(test)]
#[path = "tests/revoke_token.rs"]
mod tests;
