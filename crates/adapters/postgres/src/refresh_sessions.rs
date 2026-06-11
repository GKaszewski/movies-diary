use async_trait::async_trait;
use chrono::DateTime;
use domain::{
    errors::DomainError,
    models::RefreshSession,
    ports::RefreshSessionRepository,
    value_objects::UserId,
};
use sqlx::PgPool;

pub struct PostgresRefreshSessionAdapter {
    pool: PgPool,
}

impl PostgresRefreshSessionAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: sqlx::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl RefreshSessionRepository for PostgresRefreshSessionAdapter {
    async fn create(&self, session: &RefreshSession) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO refresh_sessions (id, user_id, token, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(session.id.to_string())
        .bind(session.user_id.value().to_string())
        .bind(&session.token)
        .bind(session.expires_at)
        .bind(session.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn get_by_token(&self, token: &str) -> Result<Option<RefreshSession>, DomainError> {
        let row = sqlx::query_as::<_, RefreshSessionRow>(
            "SELECT id, user_id, token,
                    to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS expires_at,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
             FROM refresh_sessions WHERE token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.map(RefreshSessionRow::into_domain).transpose()
    }

    async fn revoke(&self, token: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM refresh_sessions WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM refresh_sessions WHERE user_id = $1")
            .bind(user_id.value().to_string())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM refresh_sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct RefreshSessionRow {
    id: String,
    user_id: String,
    token: String,
    expires_at: String,
    created_at: String,
}

impl RefreshSessionRow {
    fn into_domain(self) -> Result<RefreshSession, DomainError> {
        let id = uuid::Uuid::parse_str(&self.id)
            .map_err(|e| DomainError::InfrastructureError(format!("invalid uuid: {e}")))?;
        let user_id = uuid::Uuid::parse_str(&self.user_id)
            .map_err(|e| DomainError::InfrastructureError(format!("invalid user_id: {e}")))?;
        let expires_at = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|e| DomainError::InfrastructureError(format!("invalid expires_at: {e}")))?
            .with_timezone(&chrono::Utc);
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| DomainError::InfrastructureError(format!("invalid created_at: {e}")))?
            .with_timezone(&chrono::Utc);
        Ok(RefreshSession {
            id,
            user_id: UserId::from_uuid(user_id),
            token: self.token,
            expires_at,
            created_at,
        })
    }
}
