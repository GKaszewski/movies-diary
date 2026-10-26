use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, WatchlistWithMovie},
    ports::LocalApContentQuery,
    value_objects::{MovieId, UserId},
};
use sqlx::SqlitePool;

pub struct SqliteApContentQuery {
    pool: SqlitePool,
}

impl SqliteApContentQuery {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

mod rows;
use rows::{DiaryRow, WatchlistRow};

#[async_trait]
impl LocalApContentQuery for SqliteApContentQuery {
    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError> {
        let uid = user_id.value().to_string();
        let rows: Vec<WatchlistRow> = sqlx::query_as(
            "SELECT w.id, w.user_id, w.movie_id, w.added_at,
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year,
                    m.director, m.poster_path
             FROM watchlist_entries w
             JOIN movies m ON m.id = w.movie_id
             WHERE w.user_id = ?
             ORDER BY w.added_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        rows.into_iter().map(WatchlistRow::into_domain).collect()
    }

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let mid = movie_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = ? AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&mid)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    async fn get_local_reviews_page(
        &self,
        user_id: &UserId,
        before: Option<chrono::NaiveDateTime>,
        limit: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let limit_i64 = limit as i64;

        let rows = if let Some(before_ts) = before {
            let ts = before_ts.format("%Y-%m-%d %H:%M:%S").to_string();
            sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ? AND r.remote_actor_url IS NULL AND r.watched_at < ?
                 ORDER BY r.watched_at DESC
                 LIMIT ?",
            )
            .bind(&uid)
            .bind(&ts)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?
        } else {
            sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url, r.watch_medium, r.spoilers
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = ? AND r.remote_actor_url IS NULL
                 ORDER BY r.watched_at DESC
                 LIMIT ?",
            )
            .bind(&uid)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?
        };
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }
}
