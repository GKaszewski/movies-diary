use domain::{
    errors::DomainError,
    value_objects::{UserId, WebhookTokenId},
};

use crate::{commands::RevokeWebhookTokenCommand, context::AppContext};

pub async fn execute(ctx: &AppContext, cmd: RevokeWebhookTokenCommand) -> Result<(), DomainError> {
    let user_id = UserId::from_uuid(cmd.user_id);
    let token_id = WebhookTokenId::from_uuid(cmd.token_id);
    ctx.webhook_token_repository
        .delete(&token_id, &user_id)
        .await
}
