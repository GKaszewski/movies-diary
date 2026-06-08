use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, Goal, Movie, Review, WatchlistWithMovie},
    ports::LocalApContentQuery,
    value_objects::{MovieId, ReviewId, UserId},
};
use sqlx::{Row, SqlitePool};

use crate::models::{DiaryRow, MovieRow, ReviewRow, WatchlistRow};

pub struct SqliteApContentQuery {
    pool: SqlitePool,
}

impl SqliteApContentQuery {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl LocalApContentQuery for SqliteApContentQuery {
    async fn get_local_reviews_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = ? AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

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
        .map_err(Self::map_err)?;
        rows.into_iter().map(WatchlistRow::into_domain).collect()
    }

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let mid = movie_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = ? AND r.remote_actor_url IS NULL
             ORDER BY r.created_at DESC",
        )
        .bind(&mid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        let id = review_id.value().to_string();
        sqlx::query_as::<_, ReviewRow>(
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at, remote_actor_url
             FROM reviews WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(ReviewRow::into_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as::<_, MovieRow>(
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::into_domain)
        .transpose()
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM reviews WHERE remote_actor_url IS NULL")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err)?;
        Ok(count as u64)
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
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
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
            .map_err(Self::map_err)?
        } else {
            sqlx::query_as::<_, DiaryRow>(
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at, r.remote_actor_url
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
            .map_err(Self::map_err)?
        };
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }

    async fn get_user_federate_goals(&self, user_id: &UserId) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let row = sqlx::query("SELECT federate_goals FROM user_settings WHERE user_id = ?")
            .bind(&uid)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::map_err)?;

        match row {
            Some(r) => {
                let val: i64 = r.try_get("federate_goals").unwrap_or(0);
                Ok(val != 0)
            }
            None => Ok(false),
        }
    }

    async fn get_goal_with_progress(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> Result<Option<(Goal, u32)>, DomainError> {
        let uid = user_id.value().to_string();
        let y = year as i64;

        let row = sqlx::query(
            "SELECT id, user_id, year, target_count, goal_type, created_at \
             FROM goals WHERE user_id = ? AND year = ?",
        )
        .bind(&uid)
        .bind(y)
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let Some(r) = row else { return Ok(None) };

        let goal = crate::goals::row_to_goal(&r)?;
        let count = crate::goals::count_reviews_in_year(&self.pool, user_id, year).await?;

        Ok(Some((goal, count)))
    }
}
