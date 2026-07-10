use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::{
    ActorRepository, FollowRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
};
use sqlx::Row;

use super::{
    PG_ACTOR_COLS, PostgresFederationRepository, pg_remote_actor, status_to_str, str_to_status,
};
use adapter_common::datetime_to_str;

#[async_trait]
impl FollowRepository for PostgresFederationRepository {
    async fn add_follower(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
        status: FollowerStatus,
        follow_activity_id: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let status_str = status_to_str(&status);
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);
        sqlx::query(
            "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at, follow_activity_id)
             VALUES ($1, $2, $3, $4::timestamptz, $5)
             ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET
                 status = EXCLUDED.status, follow_activity_id = EXCLUDED.follow_activity_id",
        ).bind(&uid).bind(remote_actor_url).bind(status_str).bind(&created_at).bind(follow_activity_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_follower_follow_activity_id(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<String> = sqlx::query_scalar(
            "SELECT follow_activity_id FROM ap_followers WHERE local_user_id = $1 AND remote_actor_url = $2",
        ).bind(&uid).bind(remote_actor_url).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    async fn remove_follower(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query("DELETE FROM ap_followers WHERE local_user_id = $1 AND remote_actor_url = $2")
            .bind(&uid)
            .bind(remote_actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_followers(&self, local_user_id: uuid::Uuid) -> Result<Vec<Follower>> {
        let uid = local_user_id.to_string();
        let q = format!(
            "SELECT f.remote_actor_url, f.status, {PG_ACTOR_COLS} FROM ap_followers f LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1"
        );
        let rows = sqlx::query(&q).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Follower {
                    actor: pg_remote_actor(row, "remote_actor_url"),
                    status: str_to_status(&status_str),
                }
            })
            .collect())
    }

    async fn get_followers_page(
        &self,
        local_user_id: uuid::Uuid,
        offset: u32,
        limit: usize,
    ) -> Result<Vec<Follower>> {
        let uid = local_user_id.to_string();
        let q = format!(
            "SELECT f.remote_actor_url, f.status, {PG_ACTOR_COLS} FROM ap_followers f LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.status = 'accepted' ORDER BY f.created_at ASC LIMIT $2 OFFSET $3"
        );
        let rows = sqlx::query(&q)
            .bind(&uid)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Follower {
                    actor: pg_remote_actor(row, "remote_actor_url"),
                    status: str_to_status(&status_str),
                }
            })
            .collect())
    }

    async fn count_followers(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = $1 AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }

    async fn update_follower_status(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
        status: FollowerStatus,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let status_str = status_to_str(&status);
        let result = sqlx::query("UPDATE ap_followers SET status = $1 WHERE local_user_id = $2 AND remote_actor_url = $3")
            .bind(status_str).bind(&uid).bind(remote_actor_url).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            tracing::warn!(local_user_id = %local_user_id, remote_actor_url, "update_follower_status: no row found");
        }
        Ok(())
    }

    async fn get_pending_followers(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();
        let q = format!(
            "SELECT f.remote_actor_url, {PG_ACTOR_COLS} FROM ap_followers f LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.status = 'pending'"
        );
        let rows = sqlx::query(&q).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| pg_remote_actor(row, "remote_actor_url"))
            .collect())
    }

    async fn get_accepted_follower_inboxes(
        &self,
        local_user_id: uuid::Uuid,
    ) -> Result<Vec<String>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT DISTINCT COALESCE(a.shared_inbox_url, a.inbox_url) as inbox
             FROM ap_followers f INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = $1 AND f.status = 'accepted'
               AND f.remote_actor_url NOT IN (SELECT remote_actor_url FROM blocked_actors WHERE local_user_id = $1)",
        ).bind(&uid).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("inbox").ok())
            .collect())
    }

    async fn count_accepted_followers(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = $1 AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }

    async fn get_accepted_followers_page(
        &self,
        local_user_id: uuid::Uuid,
        offset: u32,
        limit: usize,
    ) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();
        let q = format!(
            "SELECT f.remote_actor_url, {PG_ACTOR_COLS} FROM ap_followers f LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.status = 'accepted' ORDER BY f.created_at ASC LIMIT $2 OFFSET $3"
        );
        let rows = sqlx::query(&q)
            .bind(&uid)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|row| pg_remote_actor(row, "remote_actor_url"))
            .collect())
    }

    async fn add_following(
        &self,
        local_user_id: uuid::Uuid,
        actor: RemoteActor,
        follow_activity_id: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);
        ActorRepository::upsert_remote_actor(self, actor.clone()).await?;
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

    async fn get_following_outbox_url(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<Option<String>> = sqlx::query_scalar(
            "SELECT a.outbox_url FROM ap_following f INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url WHERE f.local_user_id = $1 AND f.remote_actor_url = $2",
        ).bind(&uid).bind(remote_actor_url).fetch_optional(&self.pool).await?;
        Ok(row.flatten())
    }

    async fn migrate_follower_actor(
        &self,
        old_actor_url: &str,
        new_actor_url: &str,
    ) -> Result<Vec<uuid::Uuid>> {
        let candidates: Vec<String> = sqlx::query_scalar(
            "SELECT local_user_id FROM ap_following WHERE remote_actor_url = $1 AND local_user_id NOT IN (SELECT local_user_id FROM ap_following WHERE remote_actor_url = $2)",
        ).bind(old_actor_url).bind(new_actor_url).fetch_all(&self.pool).await?;
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        sqlx::query("UPDATE ap_following SET remote_actor_url = $1 WHERE remote_actor_url = $2 AND local_user_id NOT IN (SELECT local_user_id FROM ap_following WHERE remote_actor_url = $1)")
            .bind(new_actor_url).bind(old_actor_url).execute(&self.pool).await?;
        candidates
            .into_iter()
            .map(|s| uuid::Uuid::parse_str(&s).map_err(|e| anyhow::anyhow!(e)))
            .collect()
    }
}
