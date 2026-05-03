use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{DiaryEntry, DiaryFilter, Movie, Review, ReviewHistory, collections::Paginated},
    ports::MovieRepository,
    value_objects::{ExternalMetadataId, MovieId, MovieTitle, ReleaseYear},
};
use sqlx::SqlitePool;

pub struct SqliteMovieRepository {
    pool: SqlitePool,
}

impl SqliteMovieRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> DomainError {
        tracing::error!("Database error: {:?}", e);
        DomainError::InfrastructureError("Database operation failed".into())
    }
}

#[async_trait::async_trait]
impl MovieRepository for SqliteMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        todo!()
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        todo!()
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        todo!()
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        todo!()
    }

    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError> {
        todo!()
    }

    async fn query_diary(
        &self,
        filter: &DiaryFilter,
    ) -> Result<Paginated<DiaryEntry>, DomainError> {
        todo!()
    }

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        todo!()
    }
}
