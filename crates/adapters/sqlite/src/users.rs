use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use domain::{
    errors::DomainError,
    models::User,
    ports::UserRepository,
    value_objects::{Email, PasswordHash, UserId},
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
}
