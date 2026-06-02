use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Row, SqlitePool};

use activitypub::RemoteReviewRepository;
use domain::models::{RemoteWatchlistEntry, Review, ReviewSource};
use domain::ports::RemoteWatchlistRepository;
use k_ap::{
    ActivityRepository, ActorRepository, BlockedDomain, BlocklistRepository, FollowRepository,
    Follower, FollowerStatus, FollowingStatus, RemoteActor,
};

fn datetime_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub struct SqliteFederationRepository {
    pool: SqlitePool,
}

impl SqliteFederationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn status_to_str(status: &FollowerStatus) -> &'static str {
    match status {
        FollowerStatus::Pending => "pending",
        FollowerStatus::Accepted => "accepted",
        FollowerStatus::Rejected => "rejected",
    }
}

fn str_to_status(s: &str) -> FollowerStatus {
    match s {
        "accepted" => FollowerStatus::Accepted,
        "rejected" => FollowerStatus::Rejected,
        _ => FollowerStatus::Pending,
    }
}

fn remote_actor_from_row(row: &sqlx::sqlite::SqliteRow, url_col: &str) -> RemoteActor {
    RemoteActor {
        url: row.get(url_col),
        handle: row.try_get("handle").unwrap_or_default(),
        inbox_url: row.try_get("inbox_url").unwrap_or_default(),
        shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
        display_name: row.try_get("display_name").ok().flatten(),
        avatar_url: row.try_get("avatar_url").ok().flatten(),
        outbox_url: row.try_get("outbox_url").ok().flatten(),
        bio: row.try_get("bio").ok().flatten(),
        banner_url: row.try_get("banner_url").ok().flatten(),
        followers_url: row.try_get("followers_url").ok().flatten(),
        following_url: row.try_get("following_url").ok().flatten(),
        also_known_as: row
            .try_get::<Option<String>, _>("also_known_as")
            .ok()
            .flatten()
            .map(|s| {
                serde_json::from_str::<Vec<String>>(&s).unwrap_or_else(|e| {
                    tracing::warn!(raw = %s, error = %e, "failed to parse also_known_as JSON");
                    vec![s]
                })
            })
            .unwrap_or_default(),
        fetched_at: row
            .try_get::<Option<String>, _>("fetched_at")
            .ok()
            .flatten()
            .and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| ndt.and_utc())
                    .or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    })
                    .ok()
            }),
    }
}

#[async_trait]
impl FollowRepository for SqliteFederationRepository {
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
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET
                 status = excluded.status,
                 follow_activity_id = excluded.follow_activity_id",
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .bind(status_str)
        .bind(&created_at)
        .bind(follow_activity_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_follower_follow_activity_id(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<Option<String>> = sqlx::query_scalar(
            "SELECT follow_activity_id FROM ap_followers WHERE local_user_id = ? AND remote_actor_url = ?",
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.flatten())
    }

    async fn remove_follower(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query("DELETE FROM ap_followers WHERE local_user_id = ? AND remote_actor_url = ?")
            .bind(&uid)
            .bind(remote_actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_followers(&self, local_user_id: uuid::Uuid) -> Result<Vec<Follower>> {
        let uid = local_user_id.to_string();

        let rows = sqlx::query(
            "SELECT f.remote_actor_url, f.status,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Follower {
                    actor: remote_actor_from_row(row, "remote_actor_url"),
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
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let rows = sqlx::query(
            "SELECT f.remote_actor_url, f.status,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'
             ORDER BY f.created_at ASC
             LIMIT ? OFFSET ?",
        )
        .bind(&uid)
        .bind(limit_i64)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| {
                let status_str: String = row.get("status");
                Follower {
                    actor: remote_actor_from_row(row, "remote_actor_url"),
                    status: str_to_status(&status_str),
                }
            })
            .collect())
    }

    async fn count_followers(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'accepted'",
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

        let result = sqlx::query(
            "UPDATE ap_followers SET status = ? WHERE local_user_id = ? AND remote_actor_url = ?",
        )
        .bind(status_str)
        .bind(&uid)
        .bind(remote_actor_url)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            tracing::warn!(local_user_id = %local_user_id, remote_actor_url, "update_follower_status: no row found");
        }

        Ok(())
    }

    async fn get_pending_followers(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();

        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'pending'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| remote_actor_from_row(row, "remote_actor_url"))
            .collect())
    }

    async fn get_accepted_follower_inboxes(
        &self,
        local_user_id: uuid::Uuid,
    ) -> Result<Vec<String>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT DISTINCT COALESCE(a.shared_inbox_url, a.inbox_url) as inbox
             FROM ap_followers f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'
               AND f.remote_actor_url NOT IN (
                   SELECT remote_actor_url FROM blocked_actors WHERE local_user_id = ?
               )",
        )
        .bind(&uid)
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("inbox").ok())
            .collect())
    }

    async fn count_accepted_followers(&self, local_user_id: uuid::Uuid) -> Result<usize> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'accepted'",
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
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'
             ORDER BY f.created_at ASC
             LIMIT ? OFFSET ?",
        )
        .bind(&uid)
        .bind(limit_i64)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| remote_actor_from_row(row, "remote_actor_url"))
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

        sqlx::query(
            "INSERT OR IGNORE INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&uid)
        .bind(&actor.url)
        .bind(follow_activity_id)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

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
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .fetch_optional(&self.pool)
        .await?;
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

    async fn get_following(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();

        let rows = sqlx::query(
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

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
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let rows = sqlx::query(
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url,
                    a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'
             ORDER BY f.created_at ASC
             LIMIT ? OFFSET ?",
        )
        .bind(&uid)
        .bind(limit_i64)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| remote_actor_from_row(row, "url"))
            .collect())
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

    async fn get_following_outbox_url(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> Result<Option<String>> {
        let uid = local_user_id.to_string();
        let row: Option<Option<String>> = sqlx::query_scalar(
            "SELECT a.outbox_url
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.remote_actor_url = ?",
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.flatten())
    }

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
        .bind(&actor.url)
        .bind(&actor.handle)
        .bind(&actor.inbox_url)
        .bind(&actor.shared_inbox_url)
        .bind(&actor.display_name)
        .bind(&actor.avatar_url)
        .bind(&actor.outbox_url)
        .bind(&actor.bio)
        .bind(&actor.banner_url)
        .bind(&actor.followers_url)
        .bind(&actor.following_url)
        .bind(&aka_json)
        .bind(&fetched_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_remote_actor(&self, actor_url: &str) -> Result<Option<RemoteActor>> {
        let row = sqlx::query(
            "SELECT url, handle, inbox_url, shared_inbox_url, display_name, avatar_url,
                    outbox_url, bio, banner_url, followers_url, following_url, also_known_as, fetched_at
             FROM ap_remote_actors WHERE url = ?",
        )
        .bind(actor_url)
        .fetch_optional(&self.pool)
        .await?;

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
            "INSERT OR IGNORE INTO ap_announces (id, object_url, actor_url, announced_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(activity_id)
        .bind(object_url)
        .bind(actor_url)
        .bind(&ts)
        .execute(&self.pool)
        .await?;
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

#[async_trait]
impl BlocklistRepository for SqliteFederationRepository {
    async fn add_blocked_domain(&self, domain: &str, reason: Option<&str>) -> Result<()> {
        let now = Utc::now().naive_utc();
        let ts = datetime_to_str(&now);
        sqlx::query(
            "INSERT INTO blocked_domains (domain, reason, blocked_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(domain) DO UPDATE SET reason = excluded.reason",
        )
        .bind(domain)
        .bind(reason)
        .bind(&ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_blocked_domain(&self, domain: &str) -> Result<()> {
        sqlx::query("DELETE FROM blocked_domains WHERE domain = ?1")
            .bind(domain)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_blocked_domains(&self) -> Result<Vec<BlockedDomain>> {
        let rows = sqlx::query(
            "SELECT domain, reason, blocked_at FROM blocked_domains ORDER BY blocked_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| BlockedDomain {
                domain: r.get("domain"),
                reason: r.get("reason"),
                blocked_at: r.get("blocked_at"),
            })
            .collect())
    }

    async fn is_domain_blocked(&self, domain: &str) -> Result<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blocked_domains WHERE domain = ?1")
                .bind(domain)
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }

    async fn add_blocked_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        let ts = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query(
            "INSERT OR IGNORE INTO blocked_actors (local_user_id, remote_actor_url, blocked_at)
             VALUES (?1, ?2, ?3)",
        )
        .bind(&uid)
        .bind(actor_url)
        .bind(&ts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_blocked_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<()> {
        let uid = local_user_id.to_string();
        sqlx::query(
            "DELETE FROM blocked_actors WHERE local_user_id = ?1 AND remote_actor_url = ?2",
        )
        .bind(&uid)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_blocked_actors(&self, local_user_id: uuid::Uuid) -> Result<Vec<String>> {
        let uid = local_user_id.to_string();
        let rows = sqlx::query(
            "SELECT remote_actor_url FROM blocked_actors WHERE local_user_id = ?1 ORDER BY blocked_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| r.get::<String, _>("remote_actor_url"))
            .collect())
    }

    async fn is_actor_blocked(&self, local_user_id: uuid::Uuid, actor_url: &str) -> Result<bool> {
        let uid = local_user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blocked_actors WHERE local_user_id = ?1 AND remote_actor_url = ?2",
        )
        .bind(&uid)
        .bind(actor_url)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }
}

#[async_trait]
impl ActivityRepository for SqliteFederationRepository {
    async fn is_activity_processed(&self, activity_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ap_activities WHERE id = ?1")
            .bind(activity_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    async fn mark_activity_processed(&self, activity_id: &str) -> Result<()> {
        let ts = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query("INSERT OR IGNORE INTO ap_activities (id, processed_at) VALUES (?1, ?2)")
            .bind(activity_id)
            .bind(&ts)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// --- Content-specific repository (movies-diary) ---

#[async_trait]
impl RemoteReviewRepository for SqliteFederationRepository {
    async fn save_remote_review(
        &self,
        review: &Review,
        ap_id: &str,
        movie_title: &str,
        release_year: u16,
        poster_url: Option<&str>,
    ) -> Result<()> {
        let actor_url = match review.source() {
            ReviewSource::Remote { actor_url } => actor_url.clone(),
            ReviewSource::Local => {
                return Err(anyhow!("save_remote_review called with a local review"));
            }
        };

        let movie_id = review.movie_id().value().to_string();

        let _ = sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, NULL, ?, ?, NULL, ?)
             ON CONFLICT(id) DO UPDATE SET
                 poster_path = COALESCE(excluded.poster_path, movies.poster_path)",
        )
        .bind(&movie_id)
        .bind(movie_title)
        .bind(release_year.max(1888) as i64)
        .bind(poster_url)
        .execute(&self.pool)
        .await?;

        let id = review.id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());

        sqlx::query(
            "INSERT OR IGNORE INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url, ap_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&movie_id)
        .bind(&user_id)
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&created_at)
        .bind(&actor_url)
        .bind(ap_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_remote_review(&self, ap_id: &str, actor_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM reviews WHERE ap_id = ? AND remote_actor_url = ?")
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_remote_review(
        &self,
        ap_id: &str,
        actor_url: &str,
        rating: u8,
        comment: Option<&str>,
        watched_at: chrono::NaiveDateTime,
    ) -> Result<()> {
        let watched_at_str = datetime_to_str(&watched_at);
        sqlx::query(
            "UPDATE reviews SET rating = ?, comment = ?, watched_at = ?
             WHERE ap_id = ? AND remote_actor_url = ?",
        )
        .bind(rating as i64)
        .bind(comment)
        .bind(&watched_at_str)
        .bind(ap_id)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_by_actor(&self, actor_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM reviews WHERE remote_actor_url = ?")
            .bind(actor_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl domain::ports::SocialQueryPort for SqliteFederationRepository {
    async fn get_accepted_following_urls(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<String>, domain::errors::DomainError> {
        let user_id_str = user_id.to_string();
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT remote_actor_url FROM ap_following
             WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&user_id_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows)
    }

    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<domain::ports::RemoteActorInfo>, domain::errors::DomainError> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT DISTINCT ar.url, ar.handle, ar.display_name
             FROM ap_remote_actors ar
             JOIN ap_following f ON f.remote_actor_url = ar.url
             WHERE f.status = 'accepted'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(url, handle, display_name)| domain::ports::RemoteActorInfo {
                    url,
                    handle,
                    display_name,
                },
            )
            .collect())
    }

    async fn count_following(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<usize, domain::errors::DomainError> {
        let uid = user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(count as usize)
    }

    async fn count_accepted_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<usize, domain::errors::DomainError> {
        let uid = user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(count as usize)
    }

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<domain::ports::PendingFollowerInfo>, domain::errors::DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
            "SELECT ar.url, ar.handle, ar.display_name, ar.avatar_url
             FROM ap_followers f
             JOIN ap_remote_actors ar ON ar.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'pending'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(
                |(url, handle, display_name, avatar_url)| domain::ports::PendingFollowerInfo {
                    url,
                    handle,
                    display_name,
                    avatar_url,
                },
            )
            .collect())
    }
}

#[async_trait]
impl RemoteWatchlistRepository for SqliteFederationRepository {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), domain::errors::DomainError> {
        sqlx::query(
            "INSERT INTO ap_remote_watchlist_entries \
             (ap_id, actor_url, movie_title, release_year, external_metadata_id, poster_url, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(ap_id) DO UPDATE SET \
               movie_title=excluded.movie_title, release_year=excluded.release_year, \
               external_metadata_id=excluded.external_metadata_id, poster_url=excluded.poster_url",
        )
        .bind(&entry.ap_id)
        .bind(&entry.actor_url)
        .bind(&entry.movie_title)
        .bind(entry.release_year as i64)
        .bind(&entry.external_metadata_id)
        .bind(&entry.poster_url)
        .bind(entry.added_at.format("%Y-%m-%d %H:%M:%S").to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn remove_by_ap_id(
        &self,
        ap_id: &str,
        actor_url: &str,
    ) -> Result<(), domain::errors::DomainError> {
        sqlx::query("DELETE FROM ap_remote_watchlist_entries WHERE ap_id = ? AND actor_url = ?")
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await
            .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, domain::errors::DomainError> {
        let rows = sqlx::query(
            "SELECT ap_id, actor_url, movie_title, release_year, external_metadata_id, poster_url, added_at \
             FROM ap_remote_watchlist_entries WHERE actor_url = ? ORDER BY added_at DESC",
        )
        .bind(actor_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let added_at_str: String = row.try_get("added_at").unwrap_or_default();
                let added_at =
                    chrono::NaiveDateTime::parse_from_str(&added_at_str, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| {
                            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                dt,
                                chrono::Utc,
                            )
                        })
                        .unwrap_or_else(|_| chrono::Utc::now());
                Ok(RemoteWatchlistEntry {
                    ap_id: row.try_get("ap_id").unwrap_or_default(),
                    actor_url: row.try_get("actor_url").unwrap_or_default(),
                    movie_title: row.try_get("movie_title").unwrap_or_default(),
                    release_year: row.try_get::<i64, _>("release_year").unwrap_or(0) as u16,
                    external_metadata_id: row.try_get("external_metadata_id").ok().flatten(),
                    poster_url: row.try_get("poster_url").ok().flatten(),
                    added_at,
                })
            })
            .collect()
    }

    async fn remove_all_by_actor(
        &self,
        actor_url: &str,
    ) -> Result<(), domain::errors::DomainError> {
        sqlx::query("DELETE FROM ap_remote_watchlist_entries WHERE actor_url = ?")
            .bind(actor_url)
            .execute(&self.pool)
            .await
            .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn get_by_derived_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, domain::errors::DomainError> {
        let actors: Vec<String> =
            sqlx::query("SELECT DISTINCT actor_url FROM ap_remote_watchlist_entries")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| domain::errors::DomainError::InfrastructureError(e.to_string()))?
                .into_iter()
                .filter_map(|row| row.try_get::<String, _>("actor_url").ok())
                .collect();

        let target = actors
            .into_iter()
            .find(|url| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, url.as_bytes()) == uuid);

        match target {
            None => Ok(vec![]),
            Some(actor_url) => self.get_by_actor_url(&actor_url).await,
        }
    }
}

pub fn wire(pool: sqlx::SqlitePool) -> activitypub::FederationRepos {
    let fed = std::sync::Arc::new(SqliteFederationRepository::new(pool));
    (
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        fed as _,
    )
}

#[cfg(test)]
mod outbox_url_tests {
    use super::*;
    use k_ap::{FollowRepository, FollowingStatus, RemoteActor};

    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE ap_remote_actors (
                url TEXT PRIMARY KEY, handle TEXT NOT NULL, inbox_url TEXT NOT NULL,
                shared_inbox_url TEXT, display_name TEXT, avatar_url TEXT,
                outbox_url TEXT, bio TEXT, banner_url TEXT, followers_url TEXT,
                following_url TEXT, also_known_as TEXT, fetched_at TEXT NOT NULL
             );
             CREATE TABLE ap_following (
                local_user_id TEXT NOT NULL, remote_actor_url TEXT NOT NULL,
                follow_activity_id TEXT, created_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                PRIMARY KEY (local_user_id, remote_actor_url)
             );",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn get_following_outbox_url_returns_stored_url() {
        let pool = setup_pool().await;
        let repo = SqliteFederationRepository::new(pool);
        let local_user = uuid::Uuid::new_v4();
        let actor = RemoteActor {
            url: "https://remote.example/users/alice".to_string(),
            handle: "alice@remote.example".to_string(),
            inbox_url: "https://remote.example/users/alice/inbox".to_string(),
            shared_inbox_url: None,
            display_name: None,
            avatar_url: None,
            outbox_url: Some("https://remote.example/users/alice/outbox".to_string()),
            bio: None,
            banner_url: None,
            followers_url: None,
            following_url: None,
            also_known_as: vec![],
            fetched_at: None,
        };
        repo.add_following(local_user, actor, "https://local/activities/1")
            .await
            .unwrap();
        repo.update_following_status(
            local_user,
            "https://remote.example/users/alice",
            FollowingStatus::Accepted,
        )
        .await
        .unwrap();

        let result = repo
            .get_following_outbox_url(local_user, "https://remote.example/users/alice")
            .await
            .unwrap();
        assert_eq!(
            result,
            Some("https://remote.example/users/alice/outbox".to_string())
        );
    }

    #[tokio::test]
    async fn get_following_outbox_url_returns_none_when_not_following() {
        let pool = setup_pool().await;
        let repo = SqliteFederationRepository::new(pool);
        let result = repo
            .get_following_outbox_url(uuid::Uuid::new_v4(), "https://remote.example/users/alice")
            .await
            .unwrap();
        assert_eq!(result, None);
    }
}

#[cfg(test)]
#[path = "tests/actor_block_tests.rs"]
mod actor_block_tests;

#[cfg(test)]
#[path = "tests/domain_block_tests.rs"]
mod domain_block_tests;

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
