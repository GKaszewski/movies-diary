use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::ImportSession,
    ports::ImportSessionRepository,
    value_objects::{ImportSessionId, UserId},
};
use sqlx::SqlitePool;

pub struct SqliteImportSessionRepository {
    pool: SqlitePool,
}

impl SqliteImportSessionRepository {
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
impl ImportSessionRepository for SqliteImportSessionRepository {
    async fn create(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let user_id = s.user_id.value().to_string();
        let created_at = s.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let expires_at = s.expires_at.format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query!(
            "INSERT INTO import_sessions (id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            id, user_id, s.parsed_data, s.field_mappings, s.row_results, created_at, expires_at
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn get(&self, id: &ImportSessionId, user_id: &UserId) -> Result<Option<ImportSession>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();
        let row = sqlx::query!(
            "SELECT id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at
             FROM import_sessions WHERE id = ? AND user_id = ?",
            id_str, uid_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(row.map(|r| -> Result<ImportSession, DomainError> {
            Ok(ImportSession {
                id: ImportSessionId::from_uuid(
                    r.id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                user_id: UserId::from_uuid(
                    r.user_id.parse::<uuid::Uuid>().map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                ),
                parsed_data: r.parsed_data,
                field_mappings: r.field_mappings,
                row_results: r.row_results,
                created_at: Self::parse_dt(&r.created_at)?,
                expires_at: Self::parse_dt(&r.expires_at)?,
            })
        }).transpose()?)
    }

    async fn update(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        sqlx::query!(
            "UPDATE import_sessions SET field_mappings = ?, row_results = ? WHERE id = ?",
            s.field_mappings, s.row_results, id
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query!("DELETE FROM import_sessions WHERE id = ?", id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query!("DELETE FROM import_sessions WHERE expires_at < datetime('now')")
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(result.rows_affected())
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query!("DELETE FROM import_sessions WHERE user_id = ? AND expires_at < datetime('now')", uid)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }
}
