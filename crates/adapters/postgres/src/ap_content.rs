use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, Movie, Review, WatchlistEntry, WatchlistWithMovie},
    ports::LocalApContentQuery,
    value_objects::{MovieId, ReviewId, UserId, WatchlistEntryId},
};
use sqlx::{PgPool, Row};

use crate::models::{DiaryRow, MovieRow, ReviewRow, parse_datetime, parse_uuid};

pub struct PostgresApContentQuery {
    pool: PgPool,
}

impl PostgresApContentQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl LocalApContentQuery for PostgresApContentQuery {
    async fn get_local_reviews_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let uid = user_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.user_id = $1 AND r.remote_actor_url IS NULL
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
        let rows = sqlx::query(
            "SELECT w.id, w.user_id, w.movie_id,
                    to_char(w.added_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS added_at,
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year,
                    m.director, m.poster_path
             FROM watchlist_entries w
             JOIN movies m ON m.id = w.movie_id
             WHERE w.user_id = $1
             ORDER BY w.added_at DESC",
        )
        .bind(&uid)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        rows.into_iter()
            .map(|row| {
                let entry = WatchlistEntry {
                    id: WatchlistEntryId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    user_id: UserId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("user_id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    movie_id: MovieId::from_uuid(parse_uuid(
                        &row.try_get::<String, _>("movie_id")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?),
                    added_at: parse_datetime(
                        &row.try_get::<String, _>("added_at")
                            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    )?,
                };
                let movie = MovieRow {
                    id: row
                        .try_get("m_id")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    external_metadata_id: row
                        .try_get("external_metadata_id")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    title: row
                        .try_get("title")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    release_year: row
                        .try_get("release_year")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    director: row
                        .try_get("director")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                    poster_path: row
                        .try_get("poster_path")
                        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?,
                }
                .into_domain()?;
                Ok(WatchlistWithMovie { entry, movie })
            })
            .collect()
    }

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError> {
        let mid = movie_id.value().to_string();
        let rows = sqlx::query_as::<_, DiaryRow>(
            "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                    r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                    to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    r.remote_actor_url
             FROM reviews r
             INNER JOIN movies m ON m.id = r.movie_id
             WHERE r.movie_id = $1 AND r.remote_actor_url IS NULL
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
            "SELECT id, movie_id, user_id, rating, comment,
                    to_char(watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                    remote_actor_url
             FROM reviews WHERE id = $1",
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
             FROM movies WHERE id = $1",
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
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                        to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                        to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                        r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND r.remote_actor_url IS NULL AND r.watched_at < $2::timestamptz
                 ORDER BY r.watched_at DESC
                 LIMIT $3",
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
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment,
                        to_char(r.watched_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS watched_at,
                        to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS created_at,
                        r.remote_actor_url
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.user_id = $1 AND r.remote_actor_url IS NULL
                 ORDER BY r.watched_at DESC
                 LIMIT $2",
            )
            .bind(&uid)
            .bind(limit_i64)
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err)?
        };
        rows.into_iter().map(DiaryRow::into_domain).collect()
    }
}
