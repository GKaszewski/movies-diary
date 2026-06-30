use async_trait::async_trait;
use domain::{errors::DomainError, models::RemoteWatchlistEntry, ports::RemoteWatchlistRepository};
use sqlx::Row;

use super::PostgresFederationRepository;

#[async_trait]
impl RemoteWatchlistRepository for PostgresFederationRepository {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO ap_remote_watchlist_entries \
             (ap_id, actor_url, movie_title, release_year, external_metadata_id, poster_url, added_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT(ap_id) DO UPDATE SET \
               movie_title=excluded.movie_title, release_year=excluded.release_year, \
               external_metadata_id=excluded.external_metadata_id, poster_url=excluded.poster_url",
        )
        .bind(&entry.ap_id).bind(&entry.actor_url).bind(&entry.movie_title)
        .bind(entry.release_year as i32).bind(&entry.external_metadata_id).bind(&entry.poster_url)
        .bind(entry.added_at)
        .execute(&self.pool).await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM ap_remote_watchlist_entries WHERE ap_id = $1 AND actor_url = $2")
            .bind(ap_id)
            .bind(actor_url)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError> {
        let rows = sqlx::query(
            "SELECT ap_id, actor_url, movie_title, release_year, external_metadata_id, poster_url, added_at \
             FROM ap_remote_watchlist_entries WHERE actor_url = $1 ORDER BY added_at DESC",
        ).bind(actor_url).fetch_all(&self.pool).await
         .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        rows.into_iter()
            .map(|row| {
                Ok(RemoteWatchlistEntry {
                    ap_id: row.try_get("ap_id").unwrap_or_default(),
                    actor_url: row.try_get("actor_url").unwrap_or_default(),
                    movie_title: row.try_get("movie_title").unwrap_or_default(),
                    release_year: row.try_get::<i32, _>("release_year").unwrap_or(0) as u16,
                    external_metadata_id: row.try_get("external_metadata_id").ok().flatten(),
                    poster_url: row.try_get("poster_url").ok().flatten(),
                    added_at: row
                        .try_get::<chrono::DateTime<chrono::Utc>, _>("added_at")
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })
            .collect()
    }

    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM ap_remote_watchlist_entries WHERE actor_url = $1")
            .bind(actor_url)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
        Ok(())
    }

    async fn get_by_derived_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError> {
        let actors: Vec<String> =
            sqlx::query("SELECT DISTINCT actor_url FROM ap_remote_watchlist_entries")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string()))?
                .into_iter()
                .filter_map(|row| row.try_get::<String, _>("actor_url").ok())
                .collect();
        let target = actors
            .into_iter()
            .find(|url| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, url.as_bytes()) == uuid);
        match target {
            None => Ok(vec![]),
            Some(actor_url) => self.get_by_actor_url(&actor_url).await,
        }
    }
}
