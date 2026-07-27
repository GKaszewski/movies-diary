use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::{FollowingReader, FollowingStatus, FollowingWriter, RemoteActor, RemoteActorCache};

use crate::{SqliteFederationRepository, remote_actor_from_row};
use adapter_common::datetime_to_str;

#[async_trait]
impl FollowingWriter for SqliteFederationRepository {
    async fn add_following(
        &self,
        local_user_id: uuid::Uuid,
        actor: RemoteActor,
        follow_activity_id: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);
        RemoteActorCache::upsert_remote_actor(self, actor.clone()).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, created_at)
             VALUES (?, ?, ?, ?)",
        ).bind(&uid).bind(&actor.url).bind(follow_activity_id).bind(&created_at).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_follow_activity_id(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<Option<String>> = sqlx::query_scalar(
            "SELECT follow_activity_id FROM ap_following WHERE local_user_id = ? AND remote_actor_url = ?",
        ).bind(&uid).bind(remote_actor_url).fetch_optional(&self.pool).await?;
        Ok(row.flatten())
    }

    async fn remove_following(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query("DELETE FROM ap_following WHERE local_user_id = ? AND remote_actor_url = ?")
            .bind(&uid)
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_following_status(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
        status: FollowingStatus,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let status_str = match status {
            FollowingStatus::Pending => "pending",
            FollowingStatus::Accepted => "accepted",
        };
        let result = sqlx::query(
            "UPDATE ap_following SET status = ? WHERE local_user_id = ? AND remote_actor_url = ?",
        )
        .bind(status_str)
        .bind(&uid)
        .bind(remote_actor_url)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            tracing::warn!(local_user_id = %local_user_id, remote_actor_url, "update_following_status: no row found");
        }
        Ok(())
    }
}

#[async_trait]
impl FollowingReader for SqliteFederationRepository {
    async fn get_following(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'",
        ).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| remote_actor_from_row(row, "url"))
            .collect())
    }

    async fn count_following(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }

    async fn get_following_page(
        &self,
        local_user_id: uuid::Uuid,
        offset: u32,
        limit: usize,
    ) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'
             ORDER BY f.created_at ASC LIMIT ? OFFSET ?",
        ).bind(&uid).bind(limit as i64).bind(offset as i64).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| remote_actor_from_row(row, "url"))
            .collect())
    }
}
