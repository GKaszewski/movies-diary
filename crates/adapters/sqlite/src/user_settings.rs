use async_trait::async_trait;
use domain::{
    errors::DomainError, models::UserSettings, ports::UserSettingsRepository, value_objects::UserId,
};
use sqlx::{Row, SqlitePool};

pub struct SqliteUserSettingsRepository {
    pool: SqlitePool,
}

impl SqliteUserSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl UserSettingsRepository for SqliteUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let uid = user_id.value().to_string();

        let row =
            sqlx::query("SELECT user_id, federate_goals FROM user_settings WHERE user_id = ?")
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

        sqlx::query("INSERT OR REPLACE INTO user_settings (user_id, federate_goals) VALUES (?, ?)")
            .bind(&uid)
            .bind(federate)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;

        Ok(())
    }
}
