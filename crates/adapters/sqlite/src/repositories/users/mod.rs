use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};

use rows::UserSummaryRow;

mod rows;
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
            domain::models::UserProfile {
                display_name: row.try_get("display_name").ok().flatten(),
                bio: row.try_get("bio").ok().flatten(),
                avatar_path: row.try_get("avatar_path").ok().flatten(),
                banner_path: row.try_get("banner_path").ok().flatten(),
                also_known_as: row.try_get("also_known_as").ok().flatten(),
                profile_fields,
            },
        ))
    }
}

const USER_COLS: &str = "id, email, username, password_hash, role, display_name, bio, avatar_path, banner_path, also_known_as";

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        let row = sqlx::query(&format!(
            "SELECT {USER_COLS} FROM users WHERE email = ? AND deleted_at IS NULL"
        ))
        .bind(email_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref()
            .map(|r| Self::row_to_user(r, vec![]))
            .transpose()
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let username_str = username.value();
        let row = sqlx::query(&format!(
            "SELECT {USER_COLS} FROM users WHERE username = ? AND deleted_at IS NULL"
        ))
        .bind(username_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref()
            .map(|r| Self::row_to_user(r, vec![]))
            .transpose()
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        if self.row_exists("email", user.email().value()).await? {
            return Err(DomainError::ValidationError(
                "Email already registered".into(),
            ));
        }
        if self.row_exists("username", user.username().value()).await? {
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
        .map_err(crate::shared::map_sqlx_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query(&format!(
            "SELECT {USER_COLS} FROM users WHERE id = ? AND deleted_at IS NULL"
        ))
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        self.load_with_fields(&id_str, row).await
    }

    async fn find_tombstoned_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        let id_str = id.value().to_string();
        let row = sqlx::query(&format!(
            "SELECT {USER_COLS} FROM users WHERE id = ? AND deleted_at IS NOT NULL"
        ))
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        self.load_with_fields(&id_str, row).await
    }

    async fn mark_deleted(
        &self,
        user_id: &UserId,
        events: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let stamped = sqlx::query(
            "UPDATE users SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL
             AND (role != 'admin' OR EXISTS (
                 SELECT 1 FROM users WHERE role = 'admin' AND deleted_at IS NULL AND id != ?2))",
        )
        .bind(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .bind(&id_str)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        if stamped.rows_affected() == 0 {
            let live: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM users WHERE id = ? AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .fetch_one(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
            return Err(if live == 0 {
                DomainError::NotFound("User not found".into())
            } else {
                DomainError::Conflict("the last active admin cannot be deleted".into())
            });
        }
        sqlx::query(
            "INSERT OR IGNORE INTO retired_usernames (username) SELECT username FROM users WHERE id = ?",
        )
        .bind(&id_str)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query("DELETE FROM refresh_sessions WHERE user_id = ?")
            .bind(&id_str)
            .execute(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        crate::shared::enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn is_username_retired(&self, username: &Username) -> Result<bool, DomainError> {
        sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(SELECT 1 FROM retired_usernames WHERE username = ?)",
        )
        .bind(username.value())
        .fetch_one(&self.pool)
        .await
        .map(|n| n != 0)
        .map_err(crate::shared::map_sqlx_error)
    }

    async fn count_active_admins(&self) -> Result<u64, DomainError> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .map(|n| n as u64)
        .map_err(crate::shared::map_sqlx_error)
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        profile: &domain::models::UserProfile,
        events: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "UPDATE users SET display_name = ?, bio = ?, avatar_path = ?, banner_path = ?, also_known_as = ? WHERE id = ?",
        )
        .bind(&profile.display_name)
        .bind(&profile.bio)
        .bind(&profile.avatar_path)
        .bind(&profile.banner_path)
        .bind(&profile.also_known_as)
        .bind(&id_str)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        crate::shared::enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        sqlx::query_as::<_, UserSummaryRow>(
            r#"SELECT u.id, u.email, u.username, u.display_name,
                      COUNT(DISTINCT r.movie_id) AS total_movies,
                      AVG(CAST(r.rating AS REAL)) AS avg_rating,
                      u.avatar_path
               FROM users u
               LEFT JOIN reviews r ON r.user_id = u.id AND r.remote_actor_url IS NULL
               WHERE u.deleted_at IS NULL
               GROUP BY u.id, u.email, u.username, u.display_name, u.avatar_path
               ORDER BY u.email ASC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .into_iter()
        .map(UserSummaryRow::into_domain)
        .collect()
    }
}

impl SqliteUserRepository {
    async fn row_exists(&self, column: &str, value: &str) -> Result<bool, DomainError> {
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE {column} = ?)"
        ))
        .bind(value)
        .fetch_one(&self.pool)
        .await
        .map(|n| n != 0)
        .map_err(crate::shared::map_sqlx_error)
    }

    async fn load_with_fields(
        &self,
        id_str: &str,
        row: Option<sqlx::sqlite::SqliteRow>,
    ) -> Result<Option<User>, DomainError> {
        let Some(r) = row else { return Ok(None) };

        let field_rows = sqlx::query(
            "SELECT name, value FROM user_profile_fields WHERE user_id = ? ORDER BY position ASC",
        )
        .bind(id_str)
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
}
