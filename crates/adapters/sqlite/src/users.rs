use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use super::models::UserSummaryRow;
use domain::{
    errors::DomainError,
    models::User,
    ports::UserRepository,
    value_objects::{Email, PasswordHash, UserId, Username},
};

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

    fn row_to_user(
        id_str: String,
        email_str: String,
        username_str: String,
        hash_str: String,
    ) -> Result<User, DomainError> {
        let id = uuid::Uuid::parse_str(&id_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let email =
            Email::new(email_str).map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let username = Username::new(username_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let hash = PasswordHash::new(hash_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(User::from_persistence(
            UserId::from_uuid(id),
            email,
            username,
            hash,
        ))
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash FROM users WHERE email = ?",
            email_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        row.map(|r| {
            Self::row_to_user(
                r.id.unwrap_or_default(),
                r.email,
                r.username,
                r.password_hash,
            )
        })
        .transpose()
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let username_str = username.value();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash FROM users WHERE username = ?",
            username_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        row.map(|r| {
            Self::row_to_user(
                r.id.unwrap_or_default(),
                r.email,
                r.username,
                r.password_hash,
            )
        })
        .transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        // Check email uniqueness first (clearer error than INSERT OR IGNORE)
        if self.find_by_email(user.email()).await?.is_some() {
            return Err(DomainError::ValidationError(
                "Email already registered".into(),
            ));
        }
        // Check username uniqueness
        if self.find_by_username(user.username()).await?.is_some() {
            return Err(DomainError::ValidationError(
                "Username already taken".into(),
            ));
        }

        let id = user.id().value().to_string();
        let email = user.email().value();
        let username = user.username().value();
        let hash = user.password_hash().value();
        let created_at = Utc::now().to_rfc3339();

        sqlx::query!(
            "INSERT INTO users (id, email, username, password_hash, created_at) VALUES (?, ?, ?, ?, ?)",
            id, email, username, hash, created_at
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash FROM users WHERE id = ?",
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        row.map(|r| {
            Self::row_to_user(
                r.id.unwrap_or_default(),
                r.email,
                r.username,
                r.password_hash,
            )
        })
        .transpose()
    }

    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        sqlx::query_as!(
            UserSummaryRow,
            r#"SELECT u.id AS "id!: String",
                      u.email AS "email!: String",
                      COUNT(DISTINCT r.movie_id) AS "total_movies!: i64",
                      AVG(CAST(r.rating AS REAL)) AS avg_rating
               FROM users u
               LEFT JOIN reviews r ON r.user_id = u.id AND r.remote_actor_url IS NULL
               GROUP BY u.id, u.email
               ORDER BY u.email ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
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
            "CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, username TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL, created_at TEXT NOT NULL)"
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
            "INSERT INTO users (id, email, username, password_hash, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind("test@example.com")
        .bind("test")
        .bind("$argon2id$v=19$m=65536,t=2,p=1$fakesalt$fakehash")
        .bind("2026-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        let result = repo.find_by_id(&UserId::from_uuid(id)).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email().value(), "test@example.com");
    }
}
