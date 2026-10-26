use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        WatchlistEntry, WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::WatchlistRepository,
    value_objects::{MovieId, UserId},
};
use sqlx::{Row, SqlitePool};

use rows::WatchlistRow;

mod rows;
use crate::shared::{datetime_to_str, enqueue_events};

pub struct SqliteWatchlistRepository {
    pool: SqlitePool,
}

impl SqliteWatchlistRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WatchlistRepository for SqliteWatchlistRepository {
    async fn add(&self, entry: &WatchlistEntry, events: &[DomainEvent]) -> Result<(), DomainError> {
        let id = entry.id.value().to_string();
        let user_id = entry.user_id.value().to_string();
        let movie_id = entry.movie_id.value().to_string();
        let added_at = datetime_to_str(&entry.added_at);

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO watchlist_entries (id, user_id, movie_id, added_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&user_id)
        .bind(&movie_id)
        .bind(&added_at)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .rows_affected()
            > 0;
        if inserted {
            enqueue_events(&mut tx, events).await?;
        }
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn remove(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let uid = user_id.value().to_string();
        let mid = movie_id.value().to_string();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let result =
            sqlx::query("DELETE FROM watchlist_entries WHERE user_id = ? AND movie_id = ?")
                .bind(&uid)
                .bind(&mid)
                .execute(&mut *tx)
                .await
                .map_err(crate::shared::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!(
                "Watchlist entry for movie {} not found",
                mid
            )));
        }
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<bool, DomainError> {
        let uid = user_id.value().to_string();
        let mid = movie_id.value().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let result =
            sqlx::query("DELETE FROM watchlist_entries WHERE user_id = ? AND movie_id = ?")
                .bind(&uid)
                .bind(&mid)
                .execute(&mut *tx)
                .await
                .map_err(crate::shared::map_sqlx_error)?;
        let removed = result.rows_affected() > 0;
        if removed {
            enqueue_events(&mut tx, events).await?;
        }
        tx.commit().await.map_err(crate::shared::map_sqlx_error)?;
        Ok(removed)
    }

    async fn get_for_user(
        &self,
        user_id: &UserId,
        page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        let uid = user_id.value().to_string();
        let limit = page.limit as i64;
        let offset = page.offset as i64;

        let rows: Vec<WatchlistRow> = sqlx::query_as(
            "SELECT w.id, w.user_id, w.movie_id, w.added_at, \
                    m.id AS m_id, m.external_metadata_id, m.title, m.release_year, \
                    m.director, m.poster_path \
             FROM watchlist_entries w \
             JOIN movies m ON m.id = w.movie_id \
             WHERE w.user_id = ? \
             ORDER BY w.added_at DESC \
             LIMIT ? OFFSET ?",
        )
        .bind(&uid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?;

        let total: i64 = sqlx::query("SELECT COUNT(*) FROM watchlist_entries WHERE user_id = ?")
            .bind(&uid)
            .fetch_one(&self.pool)
            .await
            .map_err(crate::shared::map_sqlx_error)?
            .try_get(0)
            .unwrap_or(0);

        let items = rows
            .into_iter()
            .map(|r| r.into_domain())
            .collect::<Result<Vec<_>, _>>()?;

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
        let count: i64 = sqlx::query(
            "SELECT COUNT(*) FROM watchlist_entries WHERE user_id = ? AND movie_id = ?",
        )
        .bind(&uid)
        .bind(&mid)
        .fetch_one(&self.pool)
        .await
        .map_err(crate::shared::map_sqlx_error)?
        .try_get(0)
        .unwrap_or(0);
        Ok(count > 0)
    }
}
