use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};

use super::models::UserSummaryRow;
use domain::{
    errors::DomainError,
    models::{ProfileField, User, UserRole},
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
        row: &sqlx::sqlite::SqliteRow,
        profile_fields: Vec<ProfileField>,
    ) -> Result<User, DomainError> {
        let id_str: String = row.try_get("id").unwrap_or_default();
        let id = uuid::Uuid::parse_str(&id_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let email = Email::new(row.get("email"))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let username = Username::new(row.get("username"))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let hash = PasswordHash::new(row.get("password_hash"))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let role_str: String = row.get("role");
        Ok(User::from_persistence(
            UserId::from_uuid(id),
            email,
            username,
            hash,
            Self::parse_role(&role_str),
            row.try_get("display_name").ok().flatten(),
            row.try_get("bio").ok().flatten(),
            row.try_get("avatar_path").ok().flatten(),
            row.try_get("banner_path").ok().flatten(),
            row.try_get("also_known_as").ok().flatten(),
            profile_fields,
        ))
    }
}

const USER_COLS: &str = "id, email, username, password_hash, role, display_name, bio, avatar_path, banner_path, also_known_as";

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        let row = sqlx::query(&format!("SELECT {USER_COLS} FROM users WHERE email = ?"))
            .bind(email_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::map_err)?;

        row.as_ref()
            .map(|r| Self::row_to_user(r, vec![]))
            .transpose()
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let username_str = username.value();
        let row = sqlx::query(&format!("SELECT {USER_COLS} FROM users WHERE username = ?"))
            .bind(username_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::map_err)?;

        row.as_ref()
            .map(|r| Self::row_to_user(r, vec![]))
            .transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        if self.find_by_email(user.email()).await?.is_some() {
            return Err(DomainError::ValidationError(
                "Email already registered".into(),
            ));
        }
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

        sqlx::query(
            "INSERT INTO users (id, email, username, password_hash, created_at, role) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(username)
        .bind(hash)
        .bind(&created_at)
        .bind(role)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query(&format!("SELECT {USER_COLS} FROM users WHERE id = ?"))
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::map_err)?;

        let Some(r) = row else { return Ok(None) };

        let field_rows = sqlx::query(
            "SELECT name, value FROM user_profile_fields WHERE user_id = ? ORDER BY position ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let profile_fields = field_rows
            .iter()
            .map(|f| ProfileField {
                name: f.get("name"),
                value: f.get("value"),
            })
            .collect();

        Self::row_to_user(&r, profile_fields).map(Some)
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        display_name: Option<String>,
        bio: Option<String>,
        avatar_path: Option<String>,
        banner_path: Option<String>,
        also_known_as: Option<String>,
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();
        sqlx::query(
            "UPDATE users SET display_name = ?, bio = ?, avatar_path = ?, banner_path = ?, also_known_as = ? WHERE id = ?",
        )
        .bind(&display_name)
        .bind(&bio)
        .bind(&avatar_path)
        .bind(&banner_path)
        .bind(&also_known_as)
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
                      AVG(CAST(r.rating AS REAL)) AS avg_rating,
                      u.avatar_path
               FROM users u
               LEFT JOIN reviews r ON r.user_id = u.id AND r.remote_actor_url IS NULL
               GROUP BY u.id, u.email, u.avatar_path
               ORDER BY u.email ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(UserSummaryRow::into_domain)
        .collect()
    }
}

#[cfg(test)]
#[path = "tests/users.rs"]
mod tests;
