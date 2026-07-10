use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{FederationFlags, UserSettings},
    ports::{UserFederationSettingsQuery, UserSettingsRepository},
    value_objects::UserId,
};
use sqlx::{Row, SqlitePool};

pub struct SqliteUserSettingsRepository {
    pool: SqlitePool,
}

impl SqliteUserSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl UserSettingsRepository for SqliteUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let uid = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT federate_goals, federate_reviews, federate_watchlist \
             FROM user_settings WHERE user_id = ?",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        match row {
            Some(r) => {
                let goals: i64 = r.try_get("federate_goals").unwrap_or(1);
                let reviews: i64 = r.try_get("federate_reviews").unwrap_or(1);
                let watchlist: i64 = r.try_get("federate_watchlist").unwrap_or(1);
                Ok(UserSettings::from_persistence(
                    user_id.clone(),
                    goals != 0,
                    reviews != 0,
                    watchlist != 0,
                ))
            }
            None => Ok(UserSettings::new(user_id.clone())),
        }
    }

    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError> {
        let uid = settings.user_id().value().to_string();
        sqlx::query(
            "INSERT OR REPLACE INTO user_settings \
             (user_id, federate_goals, federate_reviews, federate_watchlist) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&uid)
        .bind(if settings.federate_goals() { 1i64 } else { 0 })
        .bind(if settings.federate_reviews() { 1i64 } else { 0 })
        .bind(if settings.federate_watchlist() {
            1i64
        } else {
            0
        })
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        Ok(())
    }
}

#[async_trait]
impl UserFederationSettingsQuery for SqliteUserSettingsRepository {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError> {
        let uid = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT federate_goals, federate_reviews, federate_watchlist \
             FROM user_settings WHERE user_id = ?",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        match row {
            Some(r) => {
                let goals: i64 = r.try_get("federate_goals").unwrap_or(1);
                let reviews: i64 = r.try_get("federate_reviews").unwrap_or(1);
                let watchlist: i64 = r.try_get("federate_watchlist").unwrap_or(1);
                Ok(FederationFlags {
                    goals: goals != 0,
                    reviews: reviews != 0,
                    watchlist: watchlist != 0,
                })
            }
            None => Ok(FederationFlags {
                goals: true,
                reviews: true,
                watchlist: true,
            }),
        }
    }
}
