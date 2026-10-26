use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::{AnnotatedRow, FieldMapping, ImportSession, ParsedFile},
    ports::ImportSessionRepository,
    value_objects::{ImportSessionId, UserId},
};
use sqlx::SqlitePool;

mod rows;
use rows::*;

// ── repository ──

pub struct SqliteImportSessionRepository {
    pool: SqlitePool,
}

impl SqliteImportSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn parse_dt(s: &str) -> Result<NaiveDateTime, DomainError> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
            .map_err(|e| {
                DomainError::InfrastructureError(format!("invalid datetime '{}': {}", s, e))
            })
    }

    fn serialize_session(
        s: &ImportSession,
    ) -> Result<(String, Option<String>, Option<String>), DomainError> {
        let parsed = s
            .parsed_file
            .as_ref()
            .map(|f| {
                ser(&ParsedFileJson {
                    columns: f.columns.clone(),
                    rows: f.rows.clone(),
                })
            })
            .transpose()?
            .unwrap_or_default();
        let mappings = s
            .field_mappings
            .as_ref()
            .map(|ms| ser(&ms.iter().map(mapping_to_json).collect::<Vec<_>>()))
            .transpose()?;
        let results = s
            .row_results
            .as_ref()
            .map(|rs| ser(&rs.iter().map(annotated_to_json).collect::<Vec<_>>()))
            .transpose()?;
        Ok((parsed, mappings, results))
    }

    fn deserialize_session(
        id: String,
        user_id: String,
        parsed_data: String,
        field_mappings: Option<String>,
        row_results: Option<String>,
        created_at: &str,
        expires_at: &str,
    ) -> Result<ImportSession, DomainError> {
        let parsed_file = if parsed_data.is_empty() {
            None
        } else {
            let j: ParsedFileJson = de(&parsed_data)?;
            Some(ParsedFile {
                columns: j.columns,
                rows: j.rows,
            })
        };
        let field_mappings = field_mappings
            .as_deref()
            .map(|s| -> Result<Vec<FieldMapping>, DomainError> {
                let js: Vec<FieldMappingJson> = de(s)?;
                Ok(js.into_iter().map(mapping_from_json).collect())
            })
            .transpose()?;
        let row_results = row_results
            .as_deref()
            .map(|s| -> Result<Vec<AnnotatedRow>, DomainError> {
                let js: Vec<AnnotatedRowJson> = de(s)?;
                Ok(js.into_iter().map(annotated_from_json).collect())
            })
            .transpose()?;

        Ok(ImportSession {
            id: ImportSessionId::from_uuid(
                id.parse::<uuid::Uuid>()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
            ),
            user_id: UserId::from_uuid(
                user_id
                    .parse::<uuid::Uuid>()
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
            ),
            parsed_file,
            field_mappings,
            row_results,
            created_at: Self::parse_dt(created_at)?,
            expires_at: Self::parse_dt(expires_at)?,
        })
    }
}

#[async_trait]
impl ImportSessionRepository for SqliteImportSessionRepository {
    async fn create(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let user_id = s.user_id.value().to_string();
        let created_at = s.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let expires_at = s.expires_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let (parsed_data, field_mappings, row_results) = Self::serialize_session(s)?;
        sqlx::query(
            "INSERT INTO import_sessions (id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&parsed_data)
        .bind(&field_mappings)
        .bind(&row_results)
        .bind(&created_at)
        .bind(&expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(crate::shared::map_sqlx_error)
    }

    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at
             FROM import_sessions WHERE id = ? AND user_id = ?",
        )
        .bind(&id_str)
        .bind(&uid_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.map(|r| {
            use sqlx::Row;
            Self::deserialize_session(
                r.get("id"),
                r.get("user_id"),
                r.get("parsed_data"),
                r.get("field_mappings"),
                r.get("row_results"),
                &r.get::<String, _>("created_at"),
                &r.get::<String, _>("expires_at"),
            )
        })
        .transpose()
    }

    async fn update(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let (_, field_mappings, row_results) = Self::serialize_session(s)?;
        sqlx::query("UPDATE import_sessions SET field_mappings = ?, row_results = ? WHERE id = ?")
            .bind(&field_mappings)
            .bind(&row_results)
            .bind(&id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(crate::shared::map_sqlx_error)
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query("DELETE FROM import_sessions WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(crate::shared::map_sqlx_error)
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM import_sessions WHERE expires_at < datetime('now')")
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        Ok(result.rows_affected())
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query(
            "DELETE FROM import_sessions WHERE user_id = ? AND expires_at < datetime('now')",
        )
        .bind(&uid)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(crate::shared::map_sqlx_error)
    }
}
