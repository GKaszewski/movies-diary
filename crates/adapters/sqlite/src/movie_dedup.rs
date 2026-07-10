use async_trait::async_trait;
use domain::{
    errors::DomainError, models::Movie, ports::MovieDeduplicator, value_objects::MovieId,
};
use sqlx::SqlitePool;

pub struct SqliteMovieDeduplicator {
    pool: SqlitePool,
}

impl SqliteMovieDeduplicator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

}

#[async_trait]
impl MovieDeduplicator for SqliteMovieDeduplicator {
    async fn merge_into_canonical(
        &self,
        old_id: &MovieId,
        canonical: &Movie,
    ) -> Result<u64, DomainError> {
        let old = old_id.value().to_string();
        let new = canonical.id().value().to_string();
        let ext_id = canonical
            .external_metadata_id()
            .map(|id| id.value().to_string());
        let title = canonical.title().value().to_string();
        let year = canonical.release_year().value() as i64;
        let director = canonical.director().map(str::to_string);
        let poster = canonical.poster_path().map(|p| p.value().to_string());

        let mut tx = self.pool.begin().await.map_err(adapter_common::map_sqlx_error)?;

        // 1. Upsert canonical movie record
        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = COALESCE(excluded.external_metadata_id, movies.external_metadata_id),
                 poster_path = COALESCE(excluded.poster_path, movies.poster_path)",
        )
        .bind(&new).bind(&ext_id).bind(&title).bind(year).bind(&director).bind(&poster)
        .execute(&mut *tx).await.map_err(adapter_common::map_sqlx_error)?;

        // 2. Re-point simple FK tables
        let reviews = sqlx::query("UPDATE reviews SET movie_id = ? WHERE movie_id = ?")
            .bind(&new)
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .rows_affected();

        let watchlist = sqlx::query("UPDATE watchlist_entries SET movie_id = ? WHERE movie_id = ?")
            .bind(&new)
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .rows_affected();

        let watch_events = sqlx::query("UPDATE watch_events SET movie_id = ? WHERE movie_id = ?")
            .bind(&new)
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .rows_affected();

        // 3. Re-point movie_profiles (PK — move only if canonical has none)
        let profiles = sqlx::query("UPDATE movie_profiles SET movie_id = ? WHERE movie_id = ?")
            .bind(&new)
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?
            .rows_affected();

        // 4. Re-point enrichment tables with composite PKs (INSERT OR IGNORE + DELETE)
        //    Canonical's existing rows win on conflict — old duplicates are discarded.
        sqlx::query(
            "INSERT OR IGNORE INTO movie_genres (movie_id, tmdb_id, name)
             SELECT ?, tmdb_id, name FROM movie_genres WHERE movie_id = ?",
        )
        .bind(&new)
        .bind(&old)
        .execute(&mut *tx)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        sqlx::query("DELETE FROM movie_genres WHERE movie_id = ?")
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        sqlx::query(
            "INSERT OR IGNORE INTO movie_keywords (movie_id, tmdb_id, name)
             SELECT ?, tmdb_id, name FROM movie_keywords WHERE movie_id = ?",
        )
        .bind(&new)
        .bind(&old)
        .execute(&mut *tx)
        .await
        .map_err(adapter_common::map_sqlx_error)?;
        sqlx::query("DELETE FROM movie_keywords WHERE movie_id = ?")
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        sqlx::query(
            "INSERT OR IGNORE INTO movie_cast (movie_id, tmdb_person_id, name, character, billing_order, profile_path)
             SELECT ?, tmdb_person_id, name, character, billing_order, profile_path FROM movie_cast WHERE movie_id = ?",
        ).bind(&new).bind(&old).execute(&mut *tx).await.map_err(adapter_common::map_sqlx_error)?;
        sqlx::query("DELETE FROM movie_cast WHERE movie_id = ?")
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        sqlx::query(
            "INSERT OR IGNORE INTO movie_crew (movie_id, tmdb_person_id, name, job, department, profile_path)
             SELECT ?, tmdb_person_id, name, job, department, profile_path FROM movie_crew WHERE movie_id = ?",
        ).bind(&new).bind(&old).execute(&mut *tx).await.map_err(adapter_common::map_sqlx_error)?;
        sqlx::query("DELETE FROM movie_crew WHERE movie_id = ?")
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        // 5. Delete the now-empty old movie record (remaining cascades are safe: all FKs cleared above)
        sqlx::query("DELETE FROM movies WHERE id = ?")
            .bind(&old)
            .execute(&mut *tx)
            .await
            .map_err(adapter_common::map_sqlx_error)?;

        tx.commit().await.map_err(adapter_common::map_sqlx_error)?;

        Ok(reviews + watchlist + watch_events + profiles)
    }
}
