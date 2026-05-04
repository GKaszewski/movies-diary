use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use domain::{
    errors::DomainError,
    models::User,
    ports::UserRepository,
    value_objects::{Email, PasswordHash, UserId},
};
use super::models::UserSummaryRow;

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        let row = sqlx::query!(
            "SELECT id, email, password_hash FROM users WHERE email = ?",
            email_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let id = uuid::Uuid::parse_str(&r.id)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                let email = Email::new(r.email)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                let hash = PasswordHash::new(r.password_hash)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                Ok(Some(User::from_persistence(UserId::from_uuid(id), email, hash)))
            }
        }
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        let id = user.id().value().to_string();
        let email = user.email().value();
        let hash = user.password_hash().value();
        let created_at = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            "INSERT OR IGNORE INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?)",
            id,
            email,
            hash,
            created_at
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::ValidationError("Email already registered".into()));
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query!(
            "SELECT id, email, password_hash FROM users WHERE id = ?",
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        match row {
            None => Ok(None),
            Some(r) => {
                let uuid = uuid::Uuid::parse_str(&r.id)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                let email = Email::new(r.email)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                let hash = PasswordHash::new(r.password_hash)
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
                Ok(Some(User::from_persistence(UserId::from_uuid(uuid), email, hash)))
            }
        }
    }

    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        sqlx::query_as!(
            UserSummaryRow,
            r#"SELECT u.id,
                      u.email,
                      COUNT(r.id) AS "total_movies!: i64",
                      AVG(CAST(r.rating AS REAL)) AS avg_rating
               FROM users u
               LEFT JOIN reviews r ON r.user_id = u.id
               GROUP BY u.id, u.email
               ORDER BY u.email ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {:?}", e);
            DomainError::InfrastructureError("Database operation failed".into())
        })?
        .into_iter()
        .map(UserSummaryRow::to_domain)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup() -> (SqlitePool, SqliteUserRepository) {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TEXT NOT NULL)"
        )
        .execute(&pool)
        .await
        .unwrap();
        let repo = SqliteUserRepository::new(pool.clone());
        (pool, repo)
    }

    #[tokio::test]
    async fn find_by_id_returns_none_when_not_found() {
        let (_, repo) = setup().await;
        let result = repo
            .find_by_id(&UserId::from_uuid(uuid::Uuid::new_v4()))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn find_by_id_returns_user_when_found() {
        let (pool, repo) = setup().await;
        let id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind("test@example.com")
        .bind("$argon2id$v=19$m=65536,t=2,p=1$fakesalt$fakehash")
        .bind("2026-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        let result = repo
            .find_by_id(&UserId::from_uuid(id))
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email().value(), "test@example.com");
    }
}
