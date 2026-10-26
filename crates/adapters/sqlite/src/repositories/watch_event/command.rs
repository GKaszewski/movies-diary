use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{WatchEvent, WatchEventStatus},
    ports::WatchEventCommand,
    value_objects::WatchEventId,
};

use crate::shared::{datetime_to_str, enqueue_events};

use super::SqliteWatchEventRepository;

#[async_trait]
impl WatchEventCommand for SqliteWatchEventRepository {
    async fn save(&self, event: &WatchEvent, events: &[DomainEvent]) -> Result<(), DomainError> {
        let id = event.id().value().to_string();
        let user_id = event.user_id().value().to_string();
        let movie_id = event.movie_id().map(|m| m.value().to_string());
        let source = event.source().to_string();
        let watched_at = datetime_to_str(event.watched_at());
        let status = event.status().to_string();
        let created_at = datetime_to_str(event.created_at());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO watch_events \
             (id, user_id, movie_id, title, year, external_metadata_id, source, watched_at, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&movie_id)
        .bind(event.title())
        .bind(event.year().map(|y| y as i64))
        .bind(event.external_metadata_id())
        .bind(&source)
        .bind(&watched_at)
        .bind(&status)
        .bind(&created_at)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn update_status(
        &self,
        id: &WatchEventId,
        status: WatchEventStatus,
    ) -> Result<(), DomainError> {
        let id_str = id.value().to_string();
        let status_str = status.to_string();

        sqlx::query("UPDATE watch_events SET status = ? WHERE id = ?")
            .bind(&status_str)
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;

        Ok(())
    }

    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        status: WatchEventStatus,
    ) -> Result<u64, DomainError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE watch_events SET status = ? WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&sql).bind(status.to_string());
        for id in ids {
            q = q.bind(id.value().to_string());
        }
        let result = q
            .execute(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        Ok(result.rows_affected())
    }

    async fn delete_non_pending_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        let before_str = datetime_to_str(&before);
        let result =
            sqlx::query("DELETE FROM watch_events WHERE status != 'pending' AND created_at < ?")
                .bind(&before_str)
                .execute(&self.pool)
                .await
                .map_err(crate::shared::map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}
