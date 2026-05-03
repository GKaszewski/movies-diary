use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::{DiaryEntry, Movie, Review},
    value_objects::{
        Comment, ExternalMetadataId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
        ReviewId, UserId,
    },
};
use uuid::Uuid;

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
    pub fn to_domain(self) -> Result<Movie, DomainError> {
        let id = MovieId::from_uuid(parse_uuid(&self.id)?);
        let external_metadata_id = self
            .external_metadata_id
            .map(ExternalMetadataId::new)
            .transpose()?;
        let title = MovieTitle::new(self.title)?;
        let release_year = ReleaseYear::new(self.release_year as u16)?;
        let poster_path = self.poster_path.map(PosterPath::new).transpose()?;
        Ok(Movie::from_persistence(
            id,
            external_metadata_id,
            title,
            release_year,
            self.director,
            poster_path,
        ))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct ReviewRow {
    pub id: String,
    pub movie_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: Option<String>,
    pub watched_at: String,
    pub created_at: String,
}

impl ReviewRow {
    pub fn to_domain(self) -> Result<Review, DomainError> {
        let id = ReviewId::from_uuid(parse_uuid(&self.id)?);
        let movie_id = MovieId::from_uuid(parse_uuid(&self.movie_id)?);
        let user_id = UserId::from_uuid(parse_uuid(&self.user_id)?);
        let rating = Rating::new(self.rating as u8)?;
        let comment = self.comment.map(Comment::new).transpose()?;
        let watched_at = parse_datetime(&self.watched_at)?;
        let created_at = parse_datetime(&self.created_at)?;
        Ok(Review::from_persistence(
            id, movie_id, user_id, rating, comment, watched_at, created_at,
        ))
    }
}

// Used by query_diary JOIN — r.id aliased to review_id to avoid ambiguity with m.id
#[derive(sqlx::FromRow)]
pub(crate) struct DiaryRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub review_id: String,
    pub movie_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: Option<String>,
    pub watched_at: String,
    pub created_at: String,
}

impl DiaryRow {
    pub fn to_domain(self) -> Result<DiaryEntry, DomainError> {
        let movie = MovieRow {
            id: self.id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .to_domain()?;

        let review = ReviewRow {
            id: self.review_id,
            movie_id: self.movie_id,
            user_id: self.user_id,
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            created_at: self.created_at,
        }
        .to_domain()?;

        Ok(DiaryEntry::new(movie, review))
    }
}

pub(crate) fn parse_uuid(s: &str) -> Result<Uuid, DomainError> {
    Uuid::parse_str(s)
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid UUID '{}': {}", s, e)))
}

pub(crate) fn datetime_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn parse_datetime(s: &str) -> Result<NaiveDateTime, DomainError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| DomainError::InfrastructureError(format!("Invalid datetime '{}': {}", s, e)))
}
