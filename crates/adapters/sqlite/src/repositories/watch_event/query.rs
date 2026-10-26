use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::WatchEvent,
    ports::WatchEventQuery,
    value_objects::{UserId, WatchEventId},
};
use sqlx::Row;

use crate::shared::datetime_to_str;

use super::SqliteWatchEventRepository;
use super::rows::row_to_watch_event;

#[async_trait]
impl WatchEventQuery for SqliteWatchEventRepository {
    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError> {
        let uid = user_id.value().to_string();

        let rows = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events \
             WHERE user_id = ? AND status = 'pending' \
             ORDER BY watched_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        rows.iter().map(row_to_watch_event).collect()
    }

    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError> {
        let id_str = id.value().to_string();

        let row = sqlx::query(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        row.as_ref().map(row_to_watch_event).transpose()
    }

    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT id, user_id, movie_id, title, year, external_metadata_id, \
                    source, watched_at, status, created_at \
             FROM watch_events WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id.value().to_string());
        }
        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        rows.iter().map(row_to_watch_event).collect()
    }

    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let after_str = datetime_to_str(&after);

        let count: i64 = sqlx::query(
            "SELECT COUNT(*) FROM watch_events \
             WHERE user_id = ? AND external_metadata_id = ? AND created_at > ?",
        )
        .bind(&uid)
        .bind(external_id)
        .bind(&after_str)
        .fetch_one(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .try_get(0)
        .unwrap_or(0);

        Ok(count > 0)
    }
}
