use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::ImportProfile,
    ports::ImportProfileRepository,
    value_objects::{ImportProfileId, UserId},
};
use sqlx::PgPool;

pub struct PostgresImportProfileRepository {
    pool: PgPool,
}

impl PostgresImportProfileRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("DB error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl ImportProfileRepository for PostgresImportProfileRepository {
    async fn save(&self, p: &ImportProfile) -> Result<(), DomainError> {
        let id = p.id.value().to_string();
        let user_id = p.user_id.value().to_string();
        sqlx::query(
            "INSERT INTO import_profiles (id, user_id, name, field_mappings, created_at)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, field_mappings = EXCLUDED.field_mappings",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&p.name)
        .bind(&p.field_mappings)
        .bind(p.created_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        let uid = user_id.value().to_string();

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            user_id: String,
            name: String,
            field_mappings: String,
            created_at: NaiveDateTime,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter().map(|r| -> Result<ImportProfile, DomainError> {
            Ok(ImportProfile {
                id: ImportProfileId::from_uuid(
                    r.id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                user_id: UserId::from_uuid(
                    r.user_id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                name: r.name,
                field_mappings: r.field_mappings,
                created_at: r.created_at,
            })
        }).collect()
    }

    async fn get(&self, id: &ImportProfileId, user_id: &UserId) -> Result<Option<ImportProfile>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            user_id: String,
            name: String,
            field_mappings: String,
            created_at: NaiveDateTime,
        }

        let row = sqlx::query_as::<_, Row>(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE id = $1 AND user_id = $2",
        )
        .bind(&id_str)
        .bind(&uid_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(row.map(|r| -> Result<ImportProfile, DomainError> {
            Ok(ImportProfile {
                id: ImportProfileId::from_uuid(
                    r.id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                user_id: UserId::from_uuid(
                    r.user_id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                name: r.name,
                field_mappings: r.field_mappings,
                created_at: r.created_at,
            })
        }).transpose()?)
    }

    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query("DELETE FROM import_profiles WHERE id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }
}
