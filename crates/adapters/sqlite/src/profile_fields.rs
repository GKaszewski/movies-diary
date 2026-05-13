use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::{
    errors::DomainError,
    models::ProfileField,
    ports::UserProfileFieldsRepository,
    value_objects::UserId,
};

pub struct SqliteProfileFieldsRepository {
    pool: SqlitePool,
}

impl SqliteProfileFieldsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserProfileFieldsRepository for SqliteProfileFieldsRepository {
    async fn get_fields(&self, user_id: &UserId) -> Result<Vec<ProfileField>, DomainError> {
        let id_str = user_id.value().to_string();
        let rows = sqlx::query!(
            "SELECT name, value FROM user_profile_fields WHERE user_id = ? ORDER BY position ASC",
            id_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| ProfileField { name: r.name, value: r.value }).collect())
    }

    async fn set_fields(&self, user_id: &UserId, fields: Vec<ProfileField>) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();

        sqlx::query!("DELETE FROM user_profile_fields WHERE user_id = ?", id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        for (i, field) in fields.into_iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            let position = i as i64;
            sqlx::query!(
                "INSERT INTO user_profile_fields (id, user_id, name, value, position) VALUES (?, ?, ?, ?, ?)",
                id, id_str, field.name, field.value, position
            )
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        }

        Ok(())
    }
}
