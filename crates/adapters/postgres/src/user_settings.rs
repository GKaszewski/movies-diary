use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{FederationFlags, UserSettings},
    ports::{UserFederationSettingsQuery, UserSettingsRepository},
    value_objects::UserId,
};
use sqlx::{PgPool, Row};

pub struct PostgresUserSettingsRepository {
    pool: PgPool,
}

impl PostgresUserSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl UserSettingsRepository for PostgresUserSettingsRepository {
    async fn get(&self, user_id: &UserId) -> Result<UserSettings, DomainError> {
        let uid = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT federate_goals, federate_reviews, federate_watchlist \
             FROM user_settings WHERE user_id = $1",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        match row {
            Some(r) => {
                let goals: bool = r.try_get("federate_goals").unwrap_or(true);
                let reviews: bool = r.try_get("federate_reviews").unwrap_or(true);
                let watchlist: bool = r.try_get("federate_watchlist").unwrap_or(true);
                Ok(UserSettings::from_persistence(
                    user_id.clone(),
                    goals,
                    reviews,
                    watchlist,
                ))
            }
            None => Ok(UserSettings::new(user_id.clone())),
        }
    }

    async fn save(&self, settings: &UserSettings) -> Result<(), DomainError> {
        let uid = settings.user_id().value().to_string();
        sqlx::query(
            "INSERT INTO user_settings (user_id, federate_goals, federate_reviews, federate_watchlist) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id) DO UPDATE \
             SET federate_goals = $2, federate_reviews = $3, federate_watchlist = $4",
        )
        .bind(&uid)
        .bind(settings.federate_goals())
        .bind(settings.federate_reviews())
        .bind(settings.federate_watchlist())
        .execute(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        Ok(())
    }
}

#[async_trait]
impl UserFederationSettingsQuery for PostgresUserSettingsRepository {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError> {
        let uid = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT federate_goals, federate_reviews, federate_watchlist \
             FROM user_settings WHERE user_id = $1",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        match row {
            Some(r) => {
                let goals: bool = r.try_get("federate_goals").unwrap_or(true);
                let reviews: bool = r.try_get("federate_reviews").unwrap_or(true);
                let watchlist: bool = r.try_get("federate_watchlist").unwrap_or(true);
                Ok(FederationFlags {
                    goals,
                    reviews,
                    watchlist,
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
