use async_trait::async_trait;
use sqlx::PgPool;

use domain::{
    errors::DomainError, models::ProfileField, ports::UserProfileFieldsRepository,
    value_objects::UserId,
};

pub struct PostgresProfileFieldsRepository {
    pool: PgPool,
}

impl PostgresProfileFieldsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserProfileFieldsRepository for PostgresProfileFieldsRepository {
    async fn get_fields(&self, user_id: &UserId) -> Result<Vec<ProfileField>, DomainError> {
        let id_str = user_id.value().to_string();
        #[derive(sqlx::FromRow)]
        struct Row {
            name: String,
            value: String,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT name, value FROM user_profile_fields WHERE user_id = $1 ORDER BY position ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ProfileField {
                name: r.name,
                value: r.value,
            })
            .collect())
    }

    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<ProfileField>,
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();

        sqlx::query("DELETE FROM user_profile_fields WHERE user_id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        for (i, field) in fields.into_iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            let position = i as i64;
            sqlx::query(
                "INSERT INTO user_profile_fields (id, user_id, name, value, position) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&id)
            .bind(&id_str)
            .bind(&field.name)
            .bind(&field.value)
            .bind(position)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        }
        Ok(())
    }
}
