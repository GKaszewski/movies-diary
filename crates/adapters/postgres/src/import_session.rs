use async_trait::async_trait;
use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::ImportSession,
    ports::ImportSessionRepository,
    value_objects::{ImportSessionId, UserId},
};
use sqlx::PgPool;

pub struct PostgresImportSessionRepository {
    pool: PgPool,
}

impl PostgresImportSessionRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("DB error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl ImportSessionRepository for PostgresImportSessionRepository {
    async fn create(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        let user_id = s.user_id.value().to_string();
        sqlx::query(
            "INSERT INTO import_sessions (id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&s.parsed_data)
        .bind(&s.field_mappings)
        .bind(&s.row_results)
        .bind(s.created_at)
        .bind(s.expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn get(&self, id: &ImportSessionId, user_id: &UserId) -> Result<Option<ImportSession>, DomainError> {
        let id_str = id.value().to_string();
        let uid_str = user_id.value().to_string();

        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            user_id: String,
            parsed_data: String,
            field_mappings: Option<String>,
            row_results: Option<String>,
            created_at: NaiveDateTime,
            expires_at: NaiveDateTime,
        }

        let row = sqlx::query_as::<_, Row>(
            "SELECT id, user_id, parsed_data, field_mappings, row_results, created_at, expires_at
             FROM import_sessions WHERE id = $1 AND user_id = $2",
        )
        .bind(&id_str)
        .bind(&uid_str)
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
                created_at: r.created_at,
                expires_at: r.expires_at,
            })
        }).transpose()?)
    }

    async fn update(&self, s: &ImportSession) -> Result<(), DomainError> {
        let id = s.id.value().to_string();
        sqlx::query(
            "UPDATE import_sessions SET field_mappings = $1, row_results = $2 WHERE id = $3",
        )
        .bind(&s.field_mappings)
        .bind(&s.row_results)
        .bind(&id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Self::map_err)
    }

    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        sqlx::query("DELETE FROM import_sessions WHERE id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM import_sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(result.rows_affected())
    }

    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        sqlx::query("DELETE FROM import_sessions WHERE user_id = $1 AND expires_at < NOW()")
            .bind(&uid)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Self::map_err)
    }
}
