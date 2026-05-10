use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::ImportProfile,
    ports::ImportProfileRepository,
    value_objects::{ImportProfileId, UserId},
};
use sqlx::SqlitePool;

pub struct SqliteImportProfileRepository {
    pool: SqlitePool,
}

impl SqliteImportProfileRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("DB error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }

    fn parse_dt(s: &str) -> Result<NaiveDateTime, DomainError> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|e| DomainError::InfrastructureError(format!("invalid datetime '{}': {}", s, e)))
    }
}

#[async_trait]
impl ImportProfileRepository for SqliteImportProfileRepository {
    async fn save(&self, p: &ImportProfile) -> Result<(), DomainError> {
        let id = p.id.value().to_string();
        let user_id = p.user_id.value().to_string();
        let created_at = p.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query!(
            "INSERT OR REPLACE INTO import_profiles (id, user_id, name, field_mappings, created_at)
             VALUES (?, ?, ?, ?, ?)",
            id, user_id, p.name, p.field_mappings, created_at
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query!(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE user_id = ? ORDER BY created_at DESC",
            uid
        )
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
                created_at: Self::parse_dt(&r.created_at)?,
            })
        }).collect()
    }

    async fn get(&self, id: &ImportProfileId, user_id: &UserId) -> Result<Option<ImportProfile>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();
        let row = sqlx::query!(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE id = ? AND user_id = ?",
            id_str, uid_str
        )
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
                created_at: Self::parse_dt(&r.created_at)?,
            })
        }).transpose()?)
    }

    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query!("DELETE FROM import_profiles WHERE id = ?", id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }
}
