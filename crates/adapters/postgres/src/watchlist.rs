use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{
        WatchlistEntry, WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::WatchlistRepository,
    value_objects::{MovieId, UserId, WatchlistEntryId},
};
use sqlx::{PgPool, Row};

use crate::models::{MovieRow, parse_datetime, parse_uuid};

pub struct PostgresWatchlistRepository {
    pool: PgPool,
}

impl PostgresWatchlistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait]
impl WatchlistRepository for PostgresWatchlistRepository {
    async fn add(&self, entry: &WatchlistEntry) -> Result<(), DomainError> {
        let id = entry.id.value().to_string();
        let user_id = entry.user_id.value().to_string();
        let movie_id = entry.movie_id.value().to_string();
        let added_at = entry.added_at;

        sqlx::query(
            "INSERT INTO watchlist_entries (id, user_id, movie_id, added_at) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id, movie_id) DO NOTHING",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&movie_id)
        .bind(added_at)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn remove(&self, user_id: &UserId, movie_id: &MovieId) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        let mid = movie_id.value().to_string();

        let result =
            sqlx::query("DELETE FROM watchlist_entries WHERE user_id = $1 AND movie_id = $2")
                .bind(&uid)
                .bind(&mid)
                .execute(&self.pool)
                .await
                .map_err(Self::map_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!(
                "Watchlist entry for movie {} not found",
                mid
            )));
        }
        Ok(())
    }

    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
    ) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let mid = movie_id.value().to_string();
        let result =
            sqlx::query("DELETE FROM watchlist_entries WHERE user_id = $1 AND movie_id = $2")
                .bind(&uid)
                .bind(&mid)
                .execute(&self.pool)
                .await
                .map_err(Self::map_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_for_user(
        &self,
        user_id: &UserId,
        page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        let uid = user_id.value().to_string();
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        let rows = sqlx::query(
            "SELECT w.id, w.user_id, w.movie_id, \
                    to_char(w.added_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS added_at, \
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year, \
                    m.director, m.poster_path \
             FROM watchlist_entries w \
             JOIN movies m ON m.id = w.movie_id \
             WHERE w.user_id = $1 \
             ORDER BY w.added_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(&uid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?;

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM watchlist_entries WHERE user_id = $1")
                .bind(&uid)
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err)?;

        let items = rows
            .into_iter()
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
            .collect::<Result<Vec<_>, DomainError>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: page.limit,
            offset: page.offset,
        })
    }

    async fn contains(&self, user_id: &UserId, movie_id: &MovieId) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let mid = movie_id.value().to_string();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM watchlist_entries WHERE user_id = $1 AND movie_id = $2",
        )
        .bind(&uid)
        .bind(&mid)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(count > 0)
    }
}
