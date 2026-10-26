use async_trait::async_trait;
use domain::{
    errors::DomainError, events::DomainEvent, models::Movie, ports::MovieCommand,
    value_objects::MovieId,
};

use super::SqliteMovieRepository;
use crate::shared::enqueue_events;

#[async_trait]
impl MovieCommand for SqliteMovieRepository {
    async fn upsert_movie(&self, movie: &Movie, events: &[DomainEvent]) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        sqlx::query(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = excluded.external_metadata_id,
                 title                = excluded.title,
                 release_year         = excluded.release_year,
                 director             = excluded.director,
                 poster_path          = excluded.poster_path",
        )
        .bind(&id)
        .bind(&external_metadata_id)
        .bind(title)
        .bind(release_year)
        .bind(director)
        .bind(&poster_path)
        .execute(&mut *tx)
        .await
        .map_err(crate::shared::map_sqlx_error)?;
        enqueue_events(&mut tx, events).await?;
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }

    async fn delete_movie(
        &self,
        movie_id: &MovieId,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        let result = sqlx::query("DELETE FROM movies WHERE id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(crate::shared::map_sqlx_error)?;
        if result.rows_affected() > 0 {
            enqueue_events(&mut tx, events).await?;
        }
        tx.commit().await.map_err(crate::shared::map_sqlx_error)
    }
}
