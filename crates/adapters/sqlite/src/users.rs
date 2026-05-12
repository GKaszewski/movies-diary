use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use super::models::UserSummaryRow;
use domain::{
    errors::DomainError,
    models::{User, UserRole},
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

    fn parse_role(s: &str) -> UserRole {
        match s {
            "admin" => UserRole::Admin,
            _ => UserRole::Standard,
        }
    }

    fn row_to_user(
        id_str: String,
        email_str: String,
        username_str: String,
        hash_str: String,
        role: UserRole,
        bio: Option<String>,
        avatar_path: Option<String>,
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
            role,
            bio,
            avatar_path,
        ))
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash, role, bio, avatar_path FROM users WHERE email = ?",
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
                Self::parse_role(&r.role),
                r.bio,
                r.avatar_path,
            )
        })
        .transpose()
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let username_str = username.value();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash, role, bio, avatar_path FROM users WHERE username = ?",
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
                Self::parse_role(&r.role),
                r.bio,
                r.avatar_path,
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

        let role = match user.role() {
            UserRole::Admin => "admin",
            UserRole::Standard => "standard",
        };
        sqlx::query!(
            "INSERT INTO users (id, email, username, password_hash, created_at, role) VALUES (?, ?, ?, ?, ?, ?)",
            id, email, username, hash, created_at, role
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query!(
            "SELECT id, email, username, password_hash, role, bio, avatar_path FROM users WHERE id = ?",
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
                Self::parse_role(&r.role),
                r.bio,
                r.avatar_path,
            )
        })
        .transpose()
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        bio: Option<String>,
        avatar_path: Option<String>,
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();
        sqlx::query("UPDATE users SET bio = ?, avatar_path = ? WHERE id = ?")
            .bind(&bio)
            .bind(&avatar_path)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
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
#[path = "tests/users.rs"]
mod tests;
