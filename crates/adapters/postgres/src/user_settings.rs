use async_trait::async_trait;
use domain::{
    errors::DomainError, models::UserSettings, ports::UserSettingsRepository, value_objects::UserId,
};
use sqlx::{PgPool, Row};

pub struct PostgresUserSettingsRepository {
    pool: PgPool,
}

impl PostgresUserSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl UserSettingsRepository for PostgresUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let uid = user_id.value().to_string();

        let row =
            sqlx::query("SELECT user_id, federate_goals FROM user_settings WHERE user_id = $1")
                .bind(&uid)
                .fetch_optional(&self.pool)
                .await
                .map_err(Self::map_err)?;

        match row {
            Some(r) => {
                let federate: i64 = r.try_get("federate_goals").unwrap_or(0);
                Ok(UserSettings::from_persistence(
                    user_id.clone(),
                    federate != 0,
                ))
            }
            None => Ok(UserSettings::new(user_id.clone())),
        }
    }

    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError> {
        let uid = settings.user_id().value().to_string();
        let federate = if settings.federate_goals() { 1i64 } else { 0 };

        sqlx::query(
            "INSERT INTO user_settings (user_id, federate_goals) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET federate_goals = $2",
        )
        .bind(&uid)
        .bind(federate)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }
}
