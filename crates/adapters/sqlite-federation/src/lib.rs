use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use sqlx::{Row, SqlitePool};

use activitypub::RemoteReviewRepository;
use activitypub_base::{
    FederationRepository, Follower, FollowerStatus, FollowingStatus, RemoteActor,
};
use domain::models::{Review, ReviewSource};

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

#[async_trait]
impl FederationRepository for SqliteFederationRepository {
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
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url
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
                let shared_inbox_url: Option<String> =
                    row.try_get("shared_inbox_url").ok().flatten();
                let display_name: Option<String> = row.try_get("display_name").ok().flatten();
                let avatar_url: Option<String> = row.try_get("avatar_url").ok().flatten();

                Follower {
                    actor: RemoteActor {
                        url,
                        handle,
                        inbox_url,
                        shared_inbox_url,
                        display_name,
                        avatar_url,
                    },
                    status: str_to_status(&status_str),
                }
            })
            .collect();

        Ok(followers)
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

    async fn add_following(
        &self,
        local_user_id: uuid::Uuid,
        actor: RemoteActor,
        follow_activity_id: &str,
    ) -> Result<()> {
        let uid = local_user_id.to_string();
        let now = Utc::now().naive_utc();
        let created_at = datetime_to_str(&now);

        self.upsert_remote_actor(actor.clone()).await?;

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
            "SELECT a.url, a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url
             FROM ap_following f
             INNER JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'accepted'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| RemoteActor {
                url: row.get("url"),
                handle: row.get("handle"),
                inbox_url: row.get("inbox_url"),
                shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
                display_name: row.try_get("display_name").ok().flatten(),
                avatar_url: row.try_get("avatar_url").ok().flatten(),
            })
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

    async fn upsert_remote_actor(&self, actor: RemoteActor) -> Result<()> {
        let now = Utc::now().naive_utc();
        let fetched_at = datetime_to_str(&now);

        sqlx::query(
            "INSERT INTO ap_remote_actors (url, handle, inbox_url, shared_inbox_url, display_name, avatar_url, fetched_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
                 handle           = excluded.handle,
                 inbox_url        = excluded.inbox_url,
                 shared_inbox_url = excluded.shared_inbox_url,
                 display_name     = excluded.display_name,
                 avatar_url       = excluded.avatar_url,
                 fetched_at       = excluded.fetched_at",
        )
        .bind(&actor.url)
        .bind(&actor.handle)
        .bind(&actor.inbox_url)
        .bind(&actor.shared_inbox_url)
        .bind(&actor.display_name)
        .bind(&actor.avatar_url)
        .bind(&fetched_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_remote_actor(&self, actor_url: &str) -> Result<Option<RemoteActor>> {
        let row = sqlx::query(
            "SELECT url, handle, inbox_url, shared_inbox_url, display_name, avatar_url
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
            avatar_url: row.try_get("avatar_url").ok().flatten(),
        }))
    }

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

    async fn get_pending_followers(&self, local_user_id: uuid::Uuid) -> Result<Vec<RemoteActor>> {
        let uid = local_user_id.to_string();

        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url
             FROM ap_followers f
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ? AND f.status = 'pending'",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| RemoteActor {
                url: row.get("remote_actor_url"),
                handle: row.try_get("handle").unwrap_or_default(),
                inbox_url: row.try_get("inbox_url").unwrap_or_default(),
                shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
                display_name: row.try_get("display_name").ok().flatten(),
                avatar_url: row.try_get("avatar_url").ok().flatten(),
            })
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

    async fn count_announces(&self, object_url: &str) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM ap_announces WHERE object_url = ?1")
            .bind(object_url)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt") as usize)
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
            .map(|(url, handle, display_name)| domain::ports::RemoteActorInfo {
                url,
                handle,
                display_name,
            })
            .collect())
    }
}

pub fn wire(pool: sqlx::SqlitePool) -> (
    std::sync::Arc<dyn activitypub::FederationRepository>,
    std::sync::Arc<dyn domain::ports::SocialQueryPort>,
    std::sync::Arc<dyn activitypub::RemoteReviewRepository>,
) {
    let fed = std::sync::Arc::new(SqliteFederationRepository::new(pool));
    (
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        fed as _,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use domain::ports::SocialQueryPort;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE ap_announces (id TEXT PRIMARY KEY, object_url TEXT NOT NULL, actor_url TEXT NOT NULL, announced_at TEXT NOT NULL)")
            .execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn add_announce_stores_and_counts() {
        let pool = test_pool().await;
        let repo = SqliteFederationRepository::new(pool);
        repo.add_announce("https://remote/ann/1", "https://local/r/1", "https://remote/u/1", Utc::now()).await.unwrap();
        assert_eq!(repo.count_announces("https://local/r/1").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn duplicate_announce_is_ignored() {
        let pool = test_pool().await;
        let repo = SqliteFederationRepository::new(pool);
        repo.add_announce("https://remote/ann/1", "https://local/r/1", "https://remote/u/1", Utc::now()).await.unwrap();
        repo.add_announce("https://remote/ann/1", "https://local/r/1", "https://remote/u/1", Utc::now()).await.unwrap();
        assert_eq!(repo.count_announces("https://local/r/1").await.unwrap(), 1);
    }

    async fn setup_db(pool: &SqlitePool) {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ap_remote_actors (
                url TEXT PRIMARY KEY,
                handle TEXT NOT NULL,
                inbox_url TEXT NOT NULL,
                shared_inbox_url TEXT,
                display_name TEXT,
                avatar_url TEXT,
                fetched_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ap_following (
                local_user_id TEXT NOT NULL,
                remote_actor_url TEXT NOT NULL,
                follow_activity_id TEXT NOT NULL,
                status TEXT NOT NULL,
                PRIMARY KEY (local_user_id, remote_actor_url)
            )",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_get_accepted_following_urls_returns_only_accepted() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup_db(&pool).await;
        let repo = SqliteFederationRepository::new(pool.clone());
        let user_id = uuid::Uuid::new_v4();

        sqlx::query(
            "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
             VALUES (?, 'https://other.social/users/alice', 'act1', 'accepted'),
                    (?, 'https://other.social/users/bob', 'act2', 'pending')",
        )
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let urls = repo.get_accepted_following_urls(user_id).await.unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://other.social/users/alice");
    }

    #[tokio::test]
    async fn test_list_all_followed_remote_actors_deduplicates() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        setup_db(&pool).await;
        let repo = SqliteFederationRepository::new(pool.clone());
        let user1 = uuid::Uuid::new_v4();
        let user2 = uuid::Uuid::new_v4();

        sqlx::query(
            "INSERT INTO ap_remote_actors (url, handle, inbox_url, fetched_at, display_name)
             VALUES ('https://other.social/users/alice', 'alice@other.social', 'https://other.social/inbox', '2024-01-01', 'Alice')",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, status)
             VALUES (?, 'https://other.social/users/alice', 'act1', 'accepted'),
                    (?, 'https://other.social/users/alice', 'act2', 'accepted')",
        )
        .bind(user1.to_string())
        .bind(user2.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let actors = repo.list_all_followed_remote_actors().await.unwrap();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].handle, "alice@other.social");
    }
}
