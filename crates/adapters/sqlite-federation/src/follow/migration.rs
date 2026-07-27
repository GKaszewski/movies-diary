use anyhow::Result;
use async_trait::async_trait;
use k_ap::FollowMigration;

use crate::SqliteFederationRepository;

#[async_trait]
impl FollowMigration for SqliteFederationRepository {
    async fn migrate_follower_actor(
        &self,
        old_actor_url: &str,
        new_actor_url: &str,
    ) -> Result<Vec<uuid::Uuid>> {
        let candidates: Vec<String> = sqlx::query_scalar(
            "SELECT local_user_id FROM ap_following
             WHERE remote_actor_url = ?1
               AND local_user_id NOT IN (
                   SELECT local_user_id FROM ap_following WHERE remote_actor_url = ?2
               )",
        )
        .bind(old_actor_url)
        .bind(new_actor_url)
        .fetch_all(&self.pool)
        .await?;

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        sqlx::query(
            "UPDATE ap_following SET remote_actor_url = ?1
             WHERE remote_actor_url = ?2
               AND local_user_id NOT IN (
                   SELECT local_user_id FROM ap_following WHERE remote_actor_url = ?1
               )",
        )
        .bind(new_actor_url)
        .bind(old_actor_url)
        .execute(&self.pool)
        .await?;

        candidates
            .into_iter()
            .map(|s| uuid::Uuid::parse_str(&s).map_err(|e| anyhow::anyhow!(e)))
            .collect()
    }
}
