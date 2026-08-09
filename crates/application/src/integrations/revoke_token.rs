use domain::{
    errors::DomainError,
    value_objects::{UserId, WebhookTokenId},
};

use crate::integrations::{commands::RevokeWebhookTokenCommand, deps::RevokeWebhookTokenDeps};

pub async fn execute(
    deps: &RevokeWebhookTokenDeps,
    cmd: RevokeWebhookTokenCommand,
) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let token_id = WebhookTokenId::from_uuid(cmd.token_id);
    deps.webhook_token.delete(&token_id, &user_id).await
}

#[cfg(test)]
#[path = "tests/revoke_token.rs"]
mod tests;
