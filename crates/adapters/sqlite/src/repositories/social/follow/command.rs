use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{FollowCommand, FollowWrite},
};

use crate::shared::{datetime_to_str, enqueue_events};

use super::infra_err;
use super::rows::follow_status_to_str;
use crate::repositories::social::SqliteSocialRepository;

#[async_trait]
impl FollowCommand for SqliteSocialRepository {
    async fn apply(
        &self,
        writes: &[FollowWrite],
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(infra_err)?;
        for write in writes {
            apply_write(&mut tx, write).await?;
        }
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(infra_err)
    }
}

async fn apply_write(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    write: &FollowWrite,
) -> Result<(), DomainError> {
    let now = datetime_to_str(&Utc::now().naive_utc());
    match write {
        FollowWrite::AddFollow {
            follower_id,
            target_actor_url,
            status,
        } => {
            sqlx::query(
                "INSERT INTO ap_following (local_user_id, remote_actor_url, follow_activity_id, created_at, status)
                 VALUES (?1, ?2, '', ?3, ?4)
                 ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET status = excluded.status",
            )
            .bind(follower_id.to_string())
            .bind(target_actor_url)
            .bind(&now)
            .bind(follow_status_to_str(status))
            .execute(&mut **tx)
            .await
        }
        FollowWrite::UpdateFollowStatus {
            follower_id,
            target_actor_url,
            status,
        } => {
            sqlx::query(
                "UPDATE ap_following SET status = ?1 WHERE local_user_id = ?2 AND remote_actor_url = ?3",
            )
            .bind(follow_status_to_str(status))
            .bind(follower_id.to_string())
            .bind(target_actor_url)
            .execute(&mut **tx)
            .await
        }
        FollowWrite::RemoveFollow {
            follower_id,
            target_actor_url,
        } => {
            sqlx::query(
                "DELETE FROM ap_following WHERE local_user_id = ?1 AND remote_actor_url = ?2",
            )
            .bind(follower_id.to_string())
            .bind(target_actor_url)
            .execute(&mut **tx)
            .await
        }
        FollowWrite::AddFollower {
            local_user_id,
            follower_actor_url,
            status,
        } => {
            sqlx::query(
                "INSERT INTO ap_followers (local_user_id, remote_actor_url, status, created_at, follow_activity_id)
                 VALUES (?1, ?2, ?3, ?4, '')
                 ON CONFLICT(local_user_id, remote_actor_url) DO UPDATE SET status = excluded.status",
            )
            .bind(local_user_id.to_string())
            .bind(follower_actor_url)
            .bind(follow_status_to_str(status))
            .bind(&now)
            .execute(&mut **tx)
            .await
        }
        FollowWrite::UpdateFollowerStatus {
            local_user_id,
            follower_actor_url,
            status,
        } => {
            sqlx::query(
                "UPDATE ap_followers SET status = ?1 WHERE local_user_id = ?2 AND remote_actor_url = ?3",
            )
            .bind(follow_status_to_str(status))
            .bind(local_user_id.to_string())
            .bind(follower_actor_url)
            .execute(&mut **tx)
            .await
        }
        FollowWrite::RemoveFollowerRecord {
            local_user_id,
            follower_actor_url,
        } => {
            sqlx::query(
                "DELETE FROM ap_followers WHERE local_user_id = ?1 AND remote_actor_url = ?2",
            )
            .bind(local_user_id.to_string())
            .bind(follower_actor_url)
            .execute(&mut **tx)
            .await
        }
    }
    .map_err(infra_err)?;
    Ok(())
}
