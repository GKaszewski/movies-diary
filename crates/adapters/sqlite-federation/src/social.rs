use async_trait::async_trait;
use domain::{errors::DomainError, models::RemoteActorInfo, ports::FederationAdminQuery};

use super::SqliteFederationRepository;

#[async_trait]
impl FederationAdminQuery for SqliteFederationRepository {
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT DISTINCT ar.url, ar.handle, ar.display_name
             FROM ap_remote_actors ar
             JOIN ap_following f ON f.remote_actor_url = ar.url
             WHERE f.status = 'accepted'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|(url, handle, display_name)| RemoteActorInfo {
                url,
                handle,
                display_name,
            })
            .collect())
    }
}
