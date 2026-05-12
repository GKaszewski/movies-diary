use async_trait::async_trait;
use domain::{errors::DomainError, ports::ImageRefPort};
use sqlx::PgPool;
use std::sync::Arc;

pub struct PostgresImageRefAdapter {
    pool: PgPool,
}

impl PostgresImageRefAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub fn create_image_ref(pool: PgPool) -> Arc<dyn ImageRefPort> {
    Arc::new(PostgresImageRefAdapter::new(pool))
}

#[async_trait]
impl ImageRefPort for PostgresImageRefAdapter {
    async fn swap(&self, old_key: &str, new_key: &str) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        sqlx::query("UPDATE users SET avatar_path = $1 WHERE avatar_path = $2")
            .bind(new_key).bind(old_key)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        sqlx::query("UPDATE movies SET poster_path = $1 WHERE poster_path = $2")
            .bind(new_key).bind(old_key)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        tx.commit().await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

    async fn list_keys(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT avatar_path FROM users WHERE avatar_path IS NOT NULL
             UNION
             SELECT poster_path FROM movies WHERE poster_path IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows.into_iter().map(|(k,)| k).collect())
    }
}
