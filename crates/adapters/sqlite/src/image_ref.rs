use async_trait::async_trait;
use domain::{errors::DomainError, ports::{ImageRefCommand, ImageRefQuery}};
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct SqliteImageRefAdapter {
    pool: SqlitePool,
}

impl SqliteImageRefAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn create_image_ref(pool: SqlitePool) -> (Arc<dyn ImageRefCommand>, Arc<dyn ImageRefQuery>) {
    let adapter = Arc::new(SqliteImageRefAdapter::new(pool));
    (Arc::clone(&adapter) as Arc<dyn ImageRefCommand>, adapter as Arc<dyn ImageRefQuery>)
}

#[async_trait]
impl ImageRefCommand for SqliteImageRefAdapter {
    async fn swap(&self, old_key: &str, new_key: &str) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        sqlx::query("UPDATE users SET avatar_path = ? WHERE avatar_path = ?")
            .bind(new_key).bind(old_key)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        sqlx::query("UPDATE movies SET poster_path = ? WHERE poster_path = ?")
            .bind(new_key).bind(old_key)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        tx.commit().await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }
}

#[async_trait]
impl ImageRefQuery for SqliteImageRefAdapter {
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

#[cfg(test)]
#[path = "tests/image_ref.rs"]
mod tests;
