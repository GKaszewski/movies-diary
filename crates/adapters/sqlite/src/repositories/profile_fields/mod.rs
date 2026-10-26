use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use domain::{
    errors::DomainError, events::DomainEvent, models::ProfileField,
    ports::UserProfileFieldsRepository, value_objects::UserId,
};

use crate::shared::enqueue_events;

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
        let rows = sqlx::query(
            "SELECT name, value FROM user_profile_fields WHERE user_id = ? ORDER BY position ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        Ok(rows
            .iter()
            .map(|r| ProfileField {
                name: r.get("name"),
                value: r.get("value"),
            })
            .collect())
    }

    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<ProfileField>,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query("DELETE FROM user_profile_fields WHERE user_id = ?")
            .bind(&id_str)
            .execute(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        for (i, field) in fields.into_iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            let position = i as i64;
            sqlx::query(
                "INSERT INTO user_profile_fields (id, user_id, name, value, position) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&id_str)
            .bind(&field.name)
            .bind(&field.value)
            .bind(position)
            .execute(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        }

        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }
}
