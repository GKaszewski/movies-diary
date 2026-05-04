use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, Movie, Review, ReviewHistory, SortDirection,
        collections::Paginated,
    },
    ports::MovieRepository,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear, ReviewId},
};
use sqlx::SqlitePool;

mod migrations;
mod models;
mod users;

use models::{DiaryRow, MovieRow, ReviewRow, datetime_to_str};

pub use users::SqliteUserRepository;

pub struct SqliteMovieRepository {
    pool: SqlitePool,
}

impl SqliteMovieRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(&self) -> Result<(), DomainError> {
        migrations::run(&self.pool).await
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }

    async fn count_diary_entries(&self, movie_id: Option<&str>) -> Result<i64, DomainError> {
        match movie_id {
            None => sqlx::query_scalar!("SELECT COUNT(*) FROM reviews")
                .fetch_one(&self.pool)
                .await
                .map_err(Self::map_err),
            Some(id) => {
                sqlx::query_scalar!("SELECT COUNT(*) FROM reviews WHERE movie_id = ?", id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Self::map_err)
            }
        }
    }

    async fn fetch_diary_rows(
        &self,
        movie_id: Option<&str>,
        sort: &SortDirection,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiaryRow>, DomainError> {
        // sqlx macros require literal ORDER BY values; separate branches also let the
        // query planner use the movie_id index instead of falling back to a filtered scan.
        match (movie_id, sort) {
            (None, SortDirection::Descending) => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 ORDER BY r.watched_at DESC
                 LIMIT ? OFFSET ?",
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),

            (None, SortDirection::Ascending) => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 ORDER BY r.watched_at ASC
                 LIMIT ? OFFSET ?",
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),

            (Some(id), SortDirection::Descending) => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.movie_id = ?
                 ORDER BY r.watched_at DESC
                 LIMIT ? OFFSET ?",
                id,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),

            (Some(id), SortDirection::Ascending) => sqlx::query_as!(
                DiaryRow,
                "SELECT m.id, m.external_metadata_id, m.title, m.release_year, m.director, m.poster_path,
                        r.id AS review_id, r.movie_id, r.user_id, r.rating, r.comment, r.watched_at, r.created_at
                 FROM reviews r
                 INNER JOIN movies m ON m.id = r.movie_id
                 WHERE r.movie_id = ?
                 ORDER BY r.watched_at ASC
                 LIMIT ? OFFSET ?",
                id,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Self::map_err),
        }
    }
}

#[async_trait]
impl MovieRepository for SqliteMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let id = external_metadata_id.value();
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE external_metadata_id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::to_domain)
        .transpose()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(MovieRow::to_domain)
        .transpose()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let title = title.value();
        let year = year.value() as i64;
        sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE title = ? AND release_year = ?",
            title,
            year
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(MovieRow::to_domain)
        .collect()
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        let id = movie.id().value().to_string();
        let external_metadata_id = movie.external_metadata_id().map(|e| e.value().to_string());
        let title = movie.title().value();
        let release_year = movie.release_year().value() as i64;
        let director = movie.director();
        let poster_path = movie.poster_path().map(|p| p.value().to_string());

        sqlx::query!(
            "INSERT INTO movies (id, external_metadata_id, title, release_year, director, poster_path)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 external_metadata_id = excluded.external_metadata_id,
                 title                = excluded.title,
                 release_year         = excluded.release_year,
                 director             = excluded.director,
                 poster_path          = excluded.poster_path",
            id,
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(())
    }

    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError> {
        let id = review.id().value().to_string();
        let movie_id = review.movie_id().value().to_string();
        let user_id = review.user_id().value().to_string();
        let rating = review.rating().value() as i64;
        let comment = review.comment().map(|c| c.value().to_string());
        let watched_at = datetime_to_str(review.watched_at());
        let created_at = datetime_to_str(review.created_at());

        sqlx::query!(
            "INSERT INTO reviews (id, movie_id, user_id, rating, comment, watched_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at
        )
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;

        Ok(DomainEvent::ReviewLogged {
            review_id: review.id().clone(),
            movie_id: review.movie_id().clone(),
            user_id: review.user_id().clone(),
            rating: review.rating().clone(),
            watched_at: *review.watched_at(),
        })
    }

    async fn query_diary(&self, filter: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> {
        let movie_id: Option<String> = filter.movie_id.as_ref().map(|id| id.value().to_string());
        let limit = filter.page.limit as i64;
        let offset = filter.page.offset as i64;

        let (total, rows) = tokio::try_join!(
            self.count_diary_entries(movie_id.as_deref()),
            self.fetch_diary_rows(movie_id.as_deref(), &filter.sort_by, limit, offset)
        )?;

        let items = rows
            .into_iter()
            .map(DiaryRow::to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Paginated {
            items,
            total_count: total as u64,
            limit: filter.page.limit,
            offset: filter.page.offset,
        })
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        let id = review_id.value().to_string();
        sqlx::query_as!(
            ReviewRow,
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at
             FROM reviews WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .map(ReviewRow::to_domain)
        .transpose()
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        let id = review_id.value().to_string();
        sqlx::query!("DELETE FROM reviews WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        let id = movie_id.value().to_string();
        sqlx::query!("DELETE FROM movies WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        Ok(())
    }

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        let id_str = movie_id.value().to_string();

        let movie = sqlx::query_as!(
            MovieRow,
            "SELECT id, external_metadata_id, title, release_year, director, poster_path
             FROM movies WHERE id = ?",
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_err)?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", id_str)))?
        .to_domain()?;

        let viewings = sqlx::query_as!(
            ReviewRow,
            "SELECT id, movie_id, user_id, rating, comment, watched_at, created_at
             FROM reviews WHERE movie_id = ? ORDER BY watched_at ASC",
            id_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::map_err)?
        .into_iter()
        .map(ReviewRow::to_domain)
        .collect::<Result<Vec<_>, _>>()?;

        Ok(ReviewHistory::new(movie, viewings))
    }
}
