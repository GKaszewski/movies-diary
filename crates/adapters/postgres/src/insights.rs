use std::collections::HashMap;

use async_trait::async_trait;
use domain::{
    errors::DomainError, models::insights::WatchRow, ports::InsightsQuery, value_objects::UserId,
};
use sqlx::{PgPool, Row};

use adapter_common::{parse_datetime, parse_uuid};

use crate::wrapup::fetch_genres_pg;

/// Postgres twin of `SqliteInsightsQuery` — same shape, `$n` binds, and the
/// same thin "rows in, rows out" contract (ADR-0010).
pub struct PostgresInsightsQuery {
    pool: PgPool,
}

impl PostgresInsightsQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InsightsQuery for PostgresInsightsQuery {
    async fn get_watch_rows(&self, user_id: &UserId) -> Result<Vec<WatchRow>, DomainError> {
        let rows = sqlx::query(
            "SELECT movie_id, rating, \
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at, \
                    watch_medium \
             FROM reviews \
             WHERE user_id = $1 AND remote_actor_url IS NULL \
             ORDER BY watched_at ASC",
        )
        .bind(user_id.value().to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_common::map_sqlx_error)?;

        let mut movie_ids: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in &rows {
            let mid: String = row
                .try_get("movie_id")
                .map_err(adapter_common::map_sqlx_error)?;
            if seen.insert(mid.clone()) {
                movie_ids.push(mid);
            }
        }

        let genres_map: HashMap<String, Vec<String>> =
            fetch_genres_pg(&self.pool, &movie_ids).await?;

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
