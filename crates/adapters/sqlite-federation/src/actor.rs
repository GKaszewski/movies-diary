use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use k_ap::{ActorRepository, RemoteActor};
use sqlx::Row;

use super::{SqliteFederationRepository, remote_actor_from_row};
use adapter_common::datetime_to_str;

#[async_trait]
impl ActorRepository for SqliteFederationRepository {
    async fn get_local_actor_keypair(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<(String, String)>> {
        let uid = user_id.to_string();
        let row =
            sqlx::query("SELECT public_key, private_key FROM ap_local_actors WHERE user_id = ?")
                .bind(&uid)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| (r.get("public_key"), r.get("private_key"))))
    }

    async fn save_local_actor_keypair(
        &self,
        user_id: uuid::Uuid,
        public_key: String,
        private_key: String,
    ) -> Result<()> {
        let uid = user_id.to_string();
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);
        sqlx::query(
            "INSERT INTO ap_local_actors (user_id, public_key, private_key, created_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                 public_key  = excluded.public_key,
                 private_key = excluded.private_key",
        )
        .bind(&uid)
        .bind(&public_key)
        .bind(&private_key)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_remote_actor(&self, actor: RemoteActor) -> Result<()> {
        let now = Utc::now().naive_utc();
        let fetched_at = datetime_to_str(&now);
        let aka_json = serde_json::to_string(&actor.also_known_as).unwrap_or_default();
        sqlx::query(
            "INSERT INTO ap_remote_actors (url, handle, inbox_url, shared_inbox_url, display_name, avatar_url, outbox_url, bio, banner_url, followers_url, following_url, also_known_as, fetched_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
                 handle           = excluded.handle,
                 inbox_url        = excluded.inbox_url,
                 shared_inbox_url = excluded.shared_inbox_url,
                 display_name     = excluded.display_name,
                 avatar_url       = excluded.avatar_url,
                 outbox_url       = COALESCE(excluded.outbox_url, ap_remote_actors.outbox_url),
                 bio              = excluded.bio,
                 banner_url       = excluded.banner_url,
                 followers_url    = excluded.followers_url,
                 following_url    = excluded.following_url,
                 also_known_as    = excluded.also_known_as,
                 fetched_at       = excluded.fetched_at",
        )
        .bind(&actor.url).bind(&actor.handle).bind(&actor.inbox_url).bind(&actor.shared_inbox_url)
        .bind(&actor.display_name).bind(&actor.avatar_url).bind(&actor.outbox_url)
        .bind(&actor.bio).bind(&actor.banner_url).bind(&actor.followers_url).bind(&actor.following_url)
        .bind(&aka_json).bind(&fetched_at)
        .execute(&self.pool).await?;
        Ok(())
    }

    async fn get_remote_actor(&self, actor_url: &str) -> Result<Option<RemoteActor>> {
        let row = sqlx::query(
            "SELECT url, handle, inbox_url, shared_inbox_url, display_name, avatar_url,
                    outbox_url, bio, banner_url, followers_url, following_url, also_known_as, fetched_at
             FROM ap_remote_actors WHERE url = ?",
        ).bind(actor_url).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(|r| remote_actor_from_row(r, "url")))
    }

    async fn add_announce(
        &self,
        activity_id: &str,
        object_url: &str,
        actor_url: &str,
        announced_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let ts = announced_at.format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO ap_announces (id, object_url, actor_url, announced_at) VALUES (?1, ?2, ?3, ?4)",
        ).bind(activity_id).bind(object_url).bind(actor_url).bind(&ts).execute(&self.pool).await?;
        Ok(())
    }

    async fn remove_announce(&self, activity_id: &str, actor_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM ap_announces WHERE id = ?1 AND actor_url = ?2")
            .bind(activity_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_announces(&self, object_url: &str) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM ap_announces WHERE object_url = ?1")
            .bind(object_url)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt") as usize)
    }
}
