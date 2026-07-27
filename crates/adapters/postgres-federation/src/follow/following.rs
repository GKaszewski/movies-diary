use crate::{PG_ACTOR_COLS, PostgresFederationRepository, pg_remote_actor};
use adapter_common::datetime_to_str;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::{FollowingReader, FollowingStatus, FollowingWriter, RemoteActor, RemoteActorCache};

#[async_trait]
impl FollowingWriter for PostgresFederationRepository {
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
        sqlx::query("INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, created_at) VALUES ($1, $2, $3, $4::timestamptz) ON CONFLICT DO NOTHING")
            .bind(&uid).bind(&actor.url).bind(follow_activity_id).bind(&created_at).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_follow_activity_id(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<String> = sqlx::query_scalar("SELECT follow_activity_id FROM ap_following WHERE local_user_id = $1 AND remote_actor_url = $2")
            .bind(&uid).bind(remote_actor_url).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    async fn remove_following(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query("DELETE FROM ap_following WHERE local_user_id = $1 AND remote_actor_url = $2")
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
        let result = sqlx::query("UPDATE ap_following SET status = $1 WHERE local_user_id = $2 AND remote_actor_url = $3")
            .bind(status_str).bind(&uid).bind(remote_actor_url).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            tracing::warn!(local_user_id = %local_user_id, remote_actor_url, "update_following_status: no row found");
        }
        Ok(())
    }
}

#[async_trait]
impl FollowingReader for PostgresFederationRepository {
    async fn get_following(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();
        let q = format!(
            "SELECT a.url, {PG_ACTOR_COLS} FROM ap_following f INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.status = 'accepted'"
        );
        let rows = sqlx::query(&q).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|row| pg_remote_actor(row, "url")).collect())
    }

    async fn count_following(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = $1 AND status = 'accepted'",
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
        let q = format!(
            "SELECT a.url, {PG_ACTOR_COLS} FROM ap_following f INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.status = 'accepted' ORDER BY f.created_at ASC LIMIT $2 OFFSET $3"
        );
        let rows = sqlx::query(&q)
            .bind(&uid)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(|row| pg_remote_actor(row, "url")).collect())
    }
}
