use async_trait::async_trait;
use domain::{errors::DomainError, models::FederatedProfile, ports::FederatedProfileQuery};
use sqlx::Row;

use super::PostgresFederationRepository;

#[async_trait]
impl FederatedProfileQuery for PostgresFederationRepository {
    async fn get_federated_profile(
        &self,
        synthetic_user_id: uuid::Uuid,
    ) -> Result<Option<FederatedProfile>, DomainError> {
        let uid = synthetic_user_id.to_string();

        let actor_url: Option<String> = sqlx::query_scalar(
            "SELECT remote_actor_url FROM reviews
             WHERE user_id = $1 AND remote_actor_url IS NOT NULL
             LIMIT 1",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let actor_url = match actor_url {
            Some(url) => url,
            None => return Ok(None),
        };

        let row = sqlx::query(
            "SELECT handle, display_name, bio, avatar_url, banner_url
             FROM ap_remote_actors WHERE url = $1",
        )
        .bind(&actor_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(FederatedProfile {
                actor_url,
                handle: r.get("handle"),
                display_name: r.try_get("display_name").ok().flatten(),
                bio: r.try_get("bio").ok().flatten(),
                avatar_url: r.try_get("avatar_url").ok().flatten(),
                banner_url: r.try_get("banner_url").ok().flatten(),
            })),
            None => Ok(Some(FederatedProfile {
                handle: actor_url.clone(),
                actor_url,
                display_name: None,
                bio: None,
                avatar_url: None,
                banner_url: None,
            })),
        }
    }
}
