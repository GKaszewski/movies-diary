use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::ImportProfile,
    ports::ImportProfileRepository,
    value_objects::{ImportProfileId, UserId},
};
use sqlx::SqlitePool;

mod rows;
use rows::*;

pub struct SqliteImportProfileRepository {
    pool: SqlitePool,
}

impl SqliteImportProfileRepository {
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
}

#[async_trait]
impl ImportProfileRepository for SqliteImportProfileRepository {
    async fn save(&self, p: &ImportProfile) -> Result<(), DomainError> {
        let id = p.id.value().to_string();
        let user_id = p.user_id.value().to_string();
        let created_at = p.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let field_mappings = serialize_mappings(&p.field_mappings)?;
        sqlx::query(
            "INSERT OR REPLACE INTO import_profiles (id, user_id, name, field_mappings, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&p.name)
        .bind(&field_mappings)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(crate::shared::map_sqlx_error)
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        rows.iter()
            .map(|r| {
                use sqlx::Row;
                let id_str: String = r.get("id");
                let uid_str: String = r.get("user_id");
                let fm: String = r.get("field_mappings");
                let ca: String = r.get("created_at");
                Ok(ImportProfile {
                    id: ImportProfileId::from_uuid(
                        id_str
                            .parse::<uuid::Uuid>()
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    ),
                    user_id: UserId::from_uuid(
                        uid_str
                            .parse::<uuid::Uuid>()
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    ),
                    name: r.get("name"),
                    field_mappings: deserialize_mappings(&fm)?,
                    created_at: Self::parse_dt(&ca)?,
                })
            })
            .collect()
    }

    async fn get(
        &self,
        id: &ImportProfileId,
        user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT id, user_id, name, field_mappings, created_at FROM import_profiles WHERE id = ? AND user_id = ?",
        )
        .bind(&id_str)
        .bind(&uid_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.map(|r| {
            use sqlx::Row;
            let rid: String = r.get("id");
            let ruid: String = r.get("user_id");
            let fm: String = r.get("field_mappings");
            let ca: String = r.get("created_at");
            Ok(ImportProfile {
                id: ImportProfileId::from_uuid(
                    rid.parse::<uuid::Uuid>()
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                ),
                user_id: UserId::from_uuid(
                    ruid.parse::<uuid::Uuid>()
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                ),
                name: r.get("name"),
                field_mappings: deserialize_mappings(&fm)?,
                created_at: Self::parse_dt(&ca)?,
            })
        })
        .transpose()
    }

    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query("DELETE FROM import_profiles WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(crate::shared::map_sqlx_error)
    }
}
