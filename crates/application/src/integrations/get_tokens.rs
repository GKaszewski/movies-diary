use std::sync::Arc;

use domain::{
    errors::DomainError, models::WebhookToken, ports::WebhookTokenRepository, value_objects::UserId,
};

use crate::integrations::queries::GetWebhookTokensQuery;

pub async fn execute(
    webhook_token: Arc<dyn WebhookTokenRepository>,
    query: GetWebhookTokensQuery,
) -> Result<Vec<WebhookToken>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    webhook_token.list_by_user(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_tokens.rs"]
mod tests;
