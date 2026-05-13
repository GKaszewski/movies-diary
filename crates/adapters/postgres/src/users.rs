use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;

use domain::{
    errors::DomainError,
    models::{ProfileField, User, UserRole},
    ports::UserRepository,
    value_objects::{Email, PasswordHash, UserId, Username},
};

use super::models::UserSummaryRow;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
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
        banner_path: Option<String>,
        also_known_as: Option<String>,
        profile_fields: Vec<ProfileField>,
    ) -> Result<User, DomainError> {
        let id = uuid::Uuid::parse_str(&id_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        let email = Email::new(email_str)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
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
            banner_path,
            also_known_as,
            profile_fields,
        ))
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let email_str = email.value();
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            email: String,
            username: String,
            password_hash: String,
            role: String,
            bio: Option<String>,
            avatar_path: Option<String>,
            banner_path: Option<String>,
            also_known_as: Option<String>,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, email, username, password_hash, role, bio, avatar_path, banner_path, also_known_as FROM users WHERE email = $1",
        )
        .bind(email_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;
        row.map(|r| {
            Self::row_to_user(
                r.id,
                r.email,
                r.username,
                r.password_hash,
                Self::parse_role(&r.role),
                r.bio,
                r.avatar_path,
                r.banner_path,
                r.also_known_as,
                vec![],
            )
        })
        .transpose()
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let username_str = username.value();
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            email: String,
            username: String,
            password_hash: String,
            role: String,
            bio: Option<String>,
            avatar_path: Option<String>,
            banner_path: Option<String>,
            also_known_as: Option<String>,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, email, username, password_hash, role, bio, avatar_path, banner_path, also_known_as FROM users WHERE username = $1",
        )
        .bind(username_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;
        row.map(|r| {
            Self::row_to_user(
                r.id,
                r.email,
                r.username,
                r.password_hash,
                Self::parse_role(&r.role),
                r.bio,
                r.avatar_path,
                r.banner_path,
                r.also_known_as,
                vec![],
            )
        })
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
        let created_at = Utc::now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let role = match user.role() {
            UserRole::Admin => "admin",
            UserRole::Standard => "standard",
        };
        sqlx::query(
            "INSERT INTO users (id, email, username, password_hash, created_at, role) VALUES ($1, $2, $3, $4, $5::timestamptz, $6)",
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
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            email: String,
            username: String,
            password_hash: String,
            role: String,
            bio: Option<String>,
            avatar_path: Option<String>,
            banner_path: Option<String>,
            also_known_as: Option<String>,
        }
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, email, username, password_hash, role, bio, avatar_path, banner_path, also_known_as FROM users WHERE id = $1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let Some(r) = row else { return Ok(None) };

        #[derive(sqlx::FromRow)]
        struct FieldRow { name: String, value: String }
        let field_rows = sqlx::query_as::<_, FieldRow>(
            "SELECT name, value FROM user_profile_fields WHERE user_id = $1 ORDER BY position ASC",
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let profile_fields = field_rows.into_iter().map(|f| ProfileField { name: f.name, value: f.value }).collect();

        Self::row_to_user(
            r.id,
            r.email,
            r.username,
            r.password_hash,
            Self::parse_role(&r.role),
            r.bio,
            r.avatar_path,
            r.banner_path,
            r.also_known_as,
            profile_fields,
        ).map(Some)
    }

    async fn update_profile(
        &self,
        user_id: &UserId,
        bio: Option<String>,
        avatar_path: Option<String>,
        banner_path: Option<String>,
        also_known_as: Option<String>,
    ) -> Result<(), DomainError> {
        let id_str = user_id.value().to_string();
        sqlx::query(
            "UPDATE users SET bio = $1, avatar_path = $2, banner_path = $3, also_known_as = $4 WHERE id = $5",
        )
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
        sqlx::query_as::<_, UserSummaryRow>(
            r#"SELECT u.id, u.email,
                      COUNT(DISTINCT r.movie_id) AS total_movies,
                      AVG(r.rating::float) AS avg_rating
               FROM users u
               LEFT JOIN reviews r ON r.user_id = u.id AND r.remote_actor_url IS NULL
               GROUP BY u.id, u.email
               ORDER BY u.email ASC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(UserSummaryRow::into_domain)
        .collect()
    }
}
