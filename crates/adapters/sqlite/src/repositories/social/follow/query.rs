use async_trait::async_trait;
use domain::{
    errors::DomainError,
    value_objects::{FollowRelation, SocialActor},
};
use sqlx::Row;

use super::infra_err;
use super::rows::{follow_status_from_str, social_actor_from_row};
use crate::repositories::social::SqliteSocialRepository;

#[async_trait]
impl domain::ports::FollowQuery for SqliteSocialRepository {
    async fn get_following(&self, user_id: uuid::Uuid) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_path AS local_avatar_path,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_following f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'accepted'
               AND u.deleted_at IS NULL",
        )
        .bind(self.instance.base_url())
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, &self.instance))
            .collect())
    }

    async fn get_followers(&self, user_id: uuid::Uuid) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_path AS local_avatar_path,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_followers f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'accepted'
               AND u.deleted_at IS NULL",
        )
        .bind(self.instance.base_url())
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, &self.instance))
            .collect())
    }

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_path AS local_avatar_path,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_followers f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'pending'
               AND u.deleted_at IS NULL",
        )
        .bind(self.instance.base_url())
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, &self.instance))
            .collect())
    }

    async fn get_pending_following(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<SocialActor>, DomainError> {
        let uid = user_id.to_string();
        let rows = sqlx::query(
            "SELECT f.remote_actor_url,
                    u.username AS local_username, u.display_name AS local_display, u.avatar_path AS local_avatar_path,
                    a.handle AS remote_handle, a.display_name AS remote_display, a.avatar_url AS remote_avatar
             FROM ap_following f
             LEFT JOIN users u ON f.remote_actor_url = ?1 || '/users/' || u.id
             LEFT JOIN ap_remote_actors a ON a.url = f.remote_actor_url
             WHERE f.local_user_id = ?2 AND f.status = 'pending'
               AND u.deleted_at IS NULL",
        )
        .bind(self.instance.base_url())
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(rows
            .iter()
            .map(|r| social_actor_from_row(r, &self.instance))
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

    async fn count_pending_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError> {
        let uid = user_id.to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ap_followers WHERE local_user_id = ? AND status = 'pending'",
        )
        .bind(&uid)
        .fetch_one(&self.pool)
        .await
        .map_err(infra_err)?;
        Ok(count as usize)
    }

    async fn get_relation(
        &self,
        viewer_id: uuid::Uuid,
        target_actor_url: &str,
    ) -> Result<FollowRelation, DomainError> {
        let uid = viewer_id.to_string();
        let row = sqlx::query(
            "SELECT (SELECT status FROM ap_following WHERE local_user_id = ?1 AND remote_actor_url = ?2) AS following,
                    (SELECT status FROM ap_followers WHERE local_user_id = ?1 AND remote_actor_url = ?2) AS followed_by",
        )
        .bind(&uid)
        .bind(target_actor_url)
        .fetch_one(&self.pool)
        .await
        .map_err(infra_err)?;

        Ok(FollowRelation {
            following: row
                .try_get::<Option<String>, _>("following")
                .map_err(infra_err)?
                .as_deref()
                .and_then(follow_status_from_str),
            followed_by: row
                .try_get::<Option<String>, _>("followed_by")
                .map_err(infra_err)?
                .as_deref()
                .and_then(follow_status_from_str),
        })
    }
}
