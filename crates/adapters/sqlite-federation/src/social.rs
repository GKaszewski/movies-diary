use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{PendingFollowerInfo, RemoteActorInfo},
    ports::SocialQueryPort,
    value_objects::UserId,
};

use super::SqliteFederationRepository;

#[async_trait]
impl SocialQueryPort for SqliteFederationRepository {
    async fn get_accepted_following_urls(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<String>, DomainError> {
        let user_id_str = user_id.value().to_string();
        sqlx::query_scalar::<_, String>(
            "SELECT remote_actor_url FROM ap_following WHERE local_user_id = ? AND status = 'accepted'",
        ).bind(&user_id_str).fetch_all(&self.pool).await
         .map_err(|e| DomainError::InfrastructureError(e.to_string()))
    }

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

    async fn count_following(&self, user_id: &UserId) -> Result<usize, DomainError> {
        let uid = user_id.value().to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(count as usize)
    }

    async fn count_accepted_followers(&self, user_id: &UserId) -> Result<usize, DomainError> {
        let uid = user_id.value().to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(count as usize)
    }

    async fn get_pending_followers(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<PendingFollowerInfo>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
            "SELECT ar.url, ar.handle, ar.display_name, ar.avatar_url
             FROM ap_followers f
             JOIN ap_remote_actors ar ON ar.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'pending'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(
                |(url, handle, display_name, avatar_url)| PendingFollowerInfo {
                    url,
                    handle,
                    display_name,
                    avatar_url,
                },
            )
            .collect())
    }
}
