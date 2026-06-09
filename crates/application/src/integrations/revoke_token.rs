use domain::{
    errors::DomainError,
    value_objects::{UserId, WebhookTokenId},
};

use crate::{context::AppContext, integrations::commands::RevokeWebhookTokenCommand};

pub async fn execute(ctx: &AppContext, cmd: RevokeWebhookTokenCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let token_id = WebhookTokenId::from_uuid(cmd.token_id);
    ctx.repos.webhook_token.delete(&token_id, &user_id).await
}

#[cfg(test)]
#[path = "tests/revoke_token.rs"]
mod tests;
