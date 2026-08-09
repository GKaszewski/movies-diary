use domain::{errors::DomainError, models::WebhookToken, value_objects::UserId};

use crate::integrations::{deps::GetWebhookTokensDeps, queries::GetWebhookTokensQuery};

pub async fn execute(
    deps: &GetWebhookTokensDeps,
    query: GetWebhookTokensQuery,
) -> Result<Vec<WebhookToken>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.webhook_token.list_by_user(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_tokens.rs"]
mod tests;
