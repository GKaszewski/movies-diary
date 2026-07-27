use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    value_objects::{FollowStatus, SocialActor, SocialIdentity},
};
use sqlx::Row;

use crate::SqliteFederationRepository;
use adapter_common::datetime_to_str;

fn follow_status_to_str(status: &FollowStatus) -> &'static str {
    match status {
        FollowStatus::Pending => "pending",
        FollowStatus::Accepted => "accepted",
        FollowStatus::Rejected => "rejected",
    }
}

fn infra_err(e: impl std::fmt::Display) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl domain::ports::FollowCommand for SqliteFederationRepository {
    async fn add_follow(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let uid = follower_id.to_string();
        let status_str = follow_status_to_str(&status);
        let now = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query(
            "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, created_at, status)
             VALUES (?1, ?2, '', ?3, ?4)
             ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET status = excluded.status",
        )
        .bind(&uid)
        .bind(target_actor_url)
        .bind(&now)
        .bind(status_str)
        .execute(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(())
    }

    async fn update_follow_status(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let uid = follower_id.to_string();
        let status_str = follow_status_to_str(&status);
        sqlx::query(
            "UPDATE ap_following SET status = ?1 WHERE local_user_id = ?2 AND remote_actor_url = ?3",
        )
        .bind(status_str)
        .bind(&uid)
        .bind(target_actor_url)
        .execute(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(())
    }

    async fn remove_follow(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<(), DomainError> {
        let uid = follower_id.to_string();
        sqlx::query("DELETE FROM ap_following WHERE local_user_id = ?1 AND remote_actor_url = ?2")
            .bind(&uid)
            .bind(target_actor_url)
            .execute(&self.pool)
            .await
            .map_err(infra_err)?;
        Ok(())
    }

    async fn add_follower(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let uid = local_user_id.to_string();
        let status_str = follow_status_to_str(&status);
        let now = datetime_to_str(&Utc::now().naive_utc());
        sqlx::query(
            "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at, follow_activity_id)
             VALUES (?1, ?2, ?3, ?4, '')
             ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET status = excluded.status",
        )
        .bind(&uid)
        .bind(follower_actor_url)
        .bind(status_str)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(())
    }

    async fn update_follower_status(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let uid = local_user_id.to_string();
        let status_str = follow_status_to_str(&status);
        sqlx::query(
            "UPDATE ap_followers SET status = ?1 WHERE local_user_id = ?2 AND remote_actor_url = ?3",
        )
        .bind(status_str)
        .bind(&uid)
        .bind(follower_actor_url)
        .execute(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(())
    }

    async fn remove_follower_record(
        &self,
        local_user_id: uuid::Uuid,
        follower_actor_url: &str,
    ) -> Result<(), DomainError> {
        let uid = local_user_id.to_string();
        sqlx::query("DELETE FROM ap_followers WHERE local_user_id = ?1 AND remote_actor_url = ?2")
            .bind(&uid)
            .bind(follower_actor_url)
            .execute(&self.pool)
            .await
            .map_err(infra_err)?;
        Ok(())
    }
}

fn social_actor_from_row(row: &sqlx::sqlite::SqliteRow, base_url: &str) -> SocialActor {
    let actor_url: String = row.get("remote_actor_url");
    let identity = SocialIdentity::from_actor_url(&actor_url, base_url);

    let (handle, display_name, avatar_url) = match &identity {
        SocialIdentity::Local(_) => {
            let username: Option<String> = row.try_get("local_username").ok().flatten();
            let display: Option<String> = row.try_get("local_display").ok().flatten();
            let avatar: Option<String> = row.try_get("local_avatar").ok().flatten();
            let handle = username
                .as_deref()
                .map(|u| SocialIdentity::format_local_handle(u, base_url))
                .unwrap_or_else(|| actor_url.clone());
            (handle, display, avatar)
        }
        SocialIdentity::Remote { .. } => {
            let handle: String = row
                .try_get("remote_handle")
                .ok()
                .unwrap_or_else(|| actor_url.clone());
            let display: Option<String> = row.try_get("remote_display").ok().flatten();
            let avatar: Option<String> = row.try_get("remote_avatar").ok().flatten();
            (handle, display, avatar)
        }
    };

    SocialActor {
        identity,
        handle,
        display_name,
        avatar_url,
    }
}

#[async_trait]
impl domain::ports::FollowQuery for SqliteFederationRepository {
    async fn get_following(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_url AS local_avatar,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_following f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'accepted'",
        )
        .bind(base_url)
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, base_url))
            .collect())
    }

    async fn get_followers(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_url AS local_avatar,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_followers f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'accepted'",
        )
        .bind(base_url)
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, base_url))
            .collect())
    }

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
        base_url: &str,
    ) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_url AS local_avatar,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_followers f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'pending'",
        )
        .bind(base_url)
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, base_url))
            .collect())
    }

    async fn count_following(&self, user_id: uuid::Uuid) -> Result<usize, DomainError> {
        let uid = user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(count as usize)
    }

    async fn count_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError> {
        let uid = user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(count as usize)
    }

    async fn is_following(
        &self,
        follower_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<bool, DomainError> {
        let uid = follower_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_following WHERE local_user_id = ? AND remote_actor_url = ? AND status = 'accepted'",
        )
        .bind(&uid)
        .bind(target_actor_url)
        .fetch_one(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(count > 0)
    }
}
