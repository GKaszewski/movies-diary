use crate::shared::{movie_row_to_domain, movie_summary_to_domain};
use domain::{
    errors::DomainError,
    models::{Movie, MovieSummary},
};

#[derive(sqlx::FromRow)]
pub(crate) struct MovieRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
}

impl MovieRow {
    pub fn into_domain(self) -> Result<Movie, DomainError> {
        movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MovieSummaryRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Option<String>,
    pub runtime_minutes: Option<i64>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub collection_name: Option<String>,
}

impl MovieSummaryRow {
    pub fn into_domain(self) -> Result<MovieSummary, DomainError> {
        let movie = movie_row_to_domain(
            self.id,
            self.external_metadata_id,
            self.title,
            self.release_year,
            self.director,
            self.poster_path,
        )?;
        let genres = self
            .genres
            .map(|g| g.split(',').map(str::to_string).collect())
            .unwrap_or_default();
        Ok(movie_summary_to_domain(
            movie,
            genres,
            self.runtime_minutes,
            self.original_language,
            self.overview,
            self.collection_name,
        ))
    }
}
