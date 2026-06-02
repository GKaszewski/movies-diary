use domain::{errors::DomainError, models::WebhookToken, value_objects::UserId};

use crate::{context::AppContext, queries::GetWebhookTokensQuery};

pub async fn execute(
    ctx: &AppContext,
    query: GetWebhookTokensQuery,
) -> Result<Vec<WebhookToken>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    ctx.webhook_token_repository.list_by_user(&user_id).await
}
