use std::collections::HashMap;

use async_trait::async_trait;
use domain::{
    errors::DomainError, models::insights::WatchRow, ports::InsightsQuery, value_objects::UserId,
};
use sqlx::{Row, SqlitePool};

use adapter_common::{parse_datetime, parse_uuid};

/// SQLite read side of Insights (ADR-0010). Deliberately thin: one query for
/// review basics, one for genres — every threshold and comparison lives in
/// the pure analyzer, not here.
pub struct SqliteInsightsQuery {
    pool: SqlitePool,
}

impl SqliteInsightsQuery {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InsightsQuery for SqliteInsightsQuery {
    async fn get_watch_rows(&self, user_id: &UserId) -> Result<Vec<WatchRow>, DomainError> {
        let rows = sqlx::query(
            "SELECT movie_id, rating, watched_at, watch_medium \
             FROM reviews \
             WHERE user_id = ? AND remote_actor_url IS NULL \
             ORDER BY watched_at ASC",
        )
        .bind(user_id.value().to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        // Subquery rather than an `IN (?, …)` bind list: a whole diary can
        // exceed SQLite's bound-parameter limit.
        let genre_rows = sqlx::query(
            "SELECT movie_id, name FROM movie_genres \
             WHERE movie_id IN ( \
                 SELECT movie_id FROM reviews \
                 WHERE user_id = ? AND remote_actor_url IS NULL \
             ) \
             ORDER BY name",
        )
        .bind(user_id.value().to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        let mut genres_map: HashMap<String, Vec<String>> = HashMap::new();
        for row in genre_rows {
            let mid: String = row
                .try_get("movie_id")
                .map_err(adapter_common::map_sqlx_error)?;
            let name: String = row
                .try_get("name")
                .map_err(adapter_common::map_sqlx_error)?;
            genres_map.entry(mid).or_default().push(name);
        }

        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let movie_id: String = row
                .try_get("movie_id")
                .map_err(adapter_common::map_sqlx_error)?;
            let rating: i64 = row
                .try_get("rating")
                .map_err(adapter_common::map_sqlx_error)?;
            let watched_at: String = row
                .try_get("watched_at")
                .map_err(adapter_common::map_sqlx_error)?;
            let watch_medium: Option<String> = row
                .try_get("watch_medium")
                .map_err(adapter_common::map_sqlx_error)?;

            result.push(WatchRow {
                movie_id: parse_uuid(&movie_id)?,
                rating: rating as u8,
                watched_at: parse_datetime(&watched_at)?,
                watch_medium,
                genres: genres_map.get(&movie_id).cloned().unwrap_or_default(),
            });
        }

        Ok(result)
    }
}
