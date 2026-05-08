use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};

use activitypub::repository::{FederationRepository, Follower, FollowerStatus, RemoteActor};
use domain::models::{Review, ReviewSource};
use domain::value_objects::UserId;

use crate::models::datetime_to_str;

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

#[async_trait]
impl FederationRepository for SqliteFederationRepository {
    async fn add_follower(
        &self,
        local_user_id: UserId,
        remote_actor_url: &str,
        status: FollowerStatus,
    ) -> Result<()> {
        let uid = local_user_id.value().to_string();
        let status_str = status_to_str(&status);
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);

        sqlx::query(
            "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET status = excluded.status",
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .bind(status_str)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_follower(&self, local_user_id: UserId, remote_actor_url: &str) -> Result<()> {
        let uid = local_user_id.value().to_string();

        sqlx::query(
            "DELETE FROM ap_followers WHERE local_user_id = ? AND remote_actor_url = ?",
        )
        .bind(&uid)
        .bind(remote_actor_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_followers(&self, local_user_id: UserId) -> Result<Vec<Follower>> {
        let uid = local_user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT f.remote_actor_url, f.status,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        let followers = rows
            .into_iter()
            .map(|row| {
                let url: String = row.get("remote_actor_url");
                let status_str: String = row.get("status");
                let handle: String = row.try_get("handle").unwrap_or_default();
                let inbox_url: String = row.try_get("inbox_url").unwrap_or_default();
                let shared_inbox_url: Option<String> = row.try_get("shared_inbox_url").ok().flatten();
                let display_name: Option<String> = row.try_get("display_name").ok().flatten();

                Follower {
                    actor: RemoteActor {
                        url,
                        handle,
                        inbox_url,
                        shared_inbox_url,
                        display_name,
                    },
                    status: str_to_status(&status_str),
                }
            })
            .collect();

        Ok(followers)
    }

    async fn update_follower_status(
        &self,
        local_user_id: UserId,
        remote_actor_url: &str,
        status: FollowerStatus,
    ) -> Result<()> {
        let uid = local_user_id.value().to_string();
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
            tracing::warn!(
                local_user_id = %local_user_id.value(),
                remote_actor_url = remote_actor_url,
                "update_follower_status: no row found"
            );
        }

        Ok(())
    }

    async fn add_following(&self, local_user_id: UserId, actor: RemoteActor) -> Result<()> {
        let uid = local_user_id.value().to_string();
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);

        self.upsert_remote_actor(actor.clone()).await?;

        sqlx::query(
            "INSERT OR IGNORE INTO ap_following (local_user_id, remote_actor_url, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(&uid)
        .bind(&actor.url)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_following(&self, local_user_id: UserId, actor_url: &str) -> Result<()> {
        let uid = local_user_id.value().to_string();

        sqlx::query(
            "DELETE FROM ap_following WHERE local_user_id = ? AND remote_actor_url = ?",
        )
        .bind(&uid)
        .bind(actor_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_following(&self, local_user_id: UserId) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        let actors = rows
            .into_iter()
            .map(|row| RemoteActor {
                url: row.get("url"),
                handle: row.get("handle"),
                inbox_url: row.get("inbox_url"),
                shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
                display_name: row.try_get("display_name").ok().flatten(),
            })
            .collect();

        Ok(actors)
    }

    async fn count_following(&self, local_user_id: UserId) -> Result<usize> {
        let uid = local_user_id.value().to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ?",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }

    async fn upsert_remote_actor(&self, actor: RemoteActor) -> Result<()> {
        let now = Utc::now().naive_utc();
        let fetched_at = datetime_to_str(&now);

        sqlx::query(
            "INSERT INTO ap_remote_actors (url, handle, inbox_url, shared_inbox_url, display_name, fetched_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
                 handle           = excluded.handle,
                 inbox_url        = excluded.inbox_url,
                 shared_inbox_url = excluded.shared_inbox_url,
                 display_name     = excluded.display_name,
                 fetched_at       = excluded.fetched_at",
        )
        .bind(&actor.url)
        .bind(&actor.handle)
        .bind(&actor.inbox_url)
        .bind(&actor.shared_inbox_url)
        .bind(&actor.display_name)
        .bind(&fetched_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_remote_actor(&self, actor_url: &str) -> Result<Option<RemoteActor>> {
        let row = sqlx::query(
            "SELECT url, handle, inbox_url, shared_inbox_url, display_name
             FROM ap_remote_actors WHERE url = ?",
        )
        .bind(actor_url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| RemoteActor {
            url: row.get("url"),
            handle: row.get("handle"),
            inbox_url: row.get("inbox_url"),
            shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
            display_name: row.try_get("display_name").ok().flatten(),
        }))
    }

    async fn get_local_actor_keypair(&self, user_id: UserId) -> Result<Option<(String, String)>> {
        let uid = user_id.value().to_string();
        let row = sqlx::query(
            "SELECT public_key, private_key FROM ap_local_actors WHERE user_id = ?",
        )
        .bind(&uid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| (r.get("public_key"), r.get("private_key"))))
    }

    async fn save_local_actor_keypair(&self, user_id: UserId, public_key: String, private_key: String) -> Result<()> {
        let uid = user_id.value().to_string();
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

    async fn save_remote_review(&self, review: &Review) -> Result<()> {
        let actor_url = match review.source() {
            ReviewSource::Remote { actor_url } => actor_url.clone(),
            ReviewSource::Local => {
                return Err(anyhow!("save_remote_review called with a local review"));
            }
        };

        let id = review.id().value().to_string();
        let movie_id = review.movie_id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());

        sqlx::query(
            "INSERT OR IGNORE INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&movie_id)
        .bind(&user_id)
        .bind(rating)
        .bind(&comment)
        .bind(&watched_at)
        .bind(&created_at)
        .bind(&actor_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
