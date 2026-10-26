use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::WebhookToken,
    ports::WebhookTokenRepository,
    value_objects::{UserId, WebhookTokenId},
};
use sqlx::SqlitePool;

use crate::shared::datetime_to_str;

use super::rows::row_to_webhook_token;

pub struct SqliteWebhookTokenRepository {
    pool: SqlitePool,
}

impl SqliteWebhookTokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookTokenRepository for SqliteWebhookTokenRepository {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError> {
        let id = token.id().value().to_string();
        let user_id = token.user_id().value().to_string();
        let provider = token.provider().to_string();
        let created_at = datetime_to_str(token.created_at());
        let last_used = token.last_used_at().map(datetime_to_str);

        sqlx::query(
            "INSERT INTO webhook_tokens \
             (id, user_id, token_hash, provider, label, created_at, last_used_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(token.token_hash())
        .bind(&provider)
        .bind(token.label())
        .bind(&created_at)
        .bind(&last_used)
        .execute(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError> {
        let row = sqlx::query(
            "SELECT t.id, t.user_id, t.token_hash, t.provider, t.label, t.created_at, t.last_used_at \
             FROM webhook_tokens t JOIN users u ON u.id = t.user_id \
             WHERE t.token_hash = ? AND u.deleted_at IS NULL",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref().map(row_to_webhook_token).transpose()
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, token_hash, provider, label, created_at, last_used_at \
             FROM webhook_tokens WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        rows.iter().map(row_to_webhook_token).collect()
    }

    async fn delete(&self, id: &WebhookTokenId, user_id: &UserId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let uid = user_id.value().to_string();

        let result = sqlx::query("DELETE FROM webhook_tokens WHERE id = ? AND user_id = ?")
            .bind(&id_str)
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("Webhook token {id_str}")));
        }
        Ok(())
    }

    async fn touch_last_used(&self, id: &WebhookTokenId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let now = datetime_to_str(&chrono::Utc::now().naive_utc());

        sqlx::query("UPDATE webhook_tokens SET last_used_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        Ok(())
    }
}
