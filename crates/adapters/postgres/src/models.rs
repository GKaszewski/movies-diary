use chrono::NaiveDateTime;
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, FeedEntry, Movie, MovieSummary, PersistedReview, Review, ReviewSource,
        UserSummary,
    },
    value_objects::{
        Comment, Email, ExternalMetadataId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
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
    pub fn into_domain(self) -> Result<Movie, DomainError> {
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
pub(crate) struct MovieSummaryRow {
    pub id: String,
    pub external_metadata_id: Option<String>,
    pub title: String,
    pub release_year: i64,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub genres: Option<Vec<String>>,
    pub runtime_minutes: Option<i64>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub collection_name: Option<String>,
}

impl MovieSummaryRow {
    pub fn into_domain(self) -> Result<MovieSummary, DomainError> {
        let movie = MovieRow {
            id: self.id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .into_domain()?;
        Ok(MovieSummary {
            movie,
            genres: self.genres.unwrap_or_default(),
            runtime_minutes: self.runtime_minutes.map(|v| v as u32),
            original_language: self.original_language,
            overview: self.overview,
            collection_name: self.collection_name,
        })
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
    pub remote_actor_url: Option<String>,
}

impl ReviewRow {
    pub fn into_domain(self) -> Result<Review, DomainError> {
        let id = ReviewId::from_uuid(parse_uuid(&self.id)?);
        let movie_id = MovieId::from_uuid(parse_uuid(&self.movie_id)?);
        let user_id = UserId::from_uuid(parse_uuid(&self.user_id)?);
        let rating = Rating::new(self.rating as u8)?;
        let comment = self.comment.map(Comment::new).transpose()?;
        let watched_at = parse_datetime(&self.watched_at)?;
        let created_at = parse_datetime(&self.created_at)?;
        let source = match self.remote_actor_url {
            None => ReviewSource::Local,
            Some(url) => ReviewSource::Remote { actor_url: url },
        };
        Ok(Review::from_persistence(PersistedReview {
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            source,
        }))
    }
}

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
    pub remote_actor_url: Option<String>,
}

impl DiaryRow {
    pub fn into_domain(self) -> Result<DiaryEntry, DomainError> {
        let movie = MovieRow {
            id: self.id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .into_domain()?;
        let review = ReviewRow {
            id: self.review_id,
            movie_id: self.movie_id,
            user_id: self.user_id,
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            created_at: self.created_at,
            remote_actor_url: self.remote_actor_url,
        }
        .into_domain()?;
        Ok(DiaryEntry::new(movie, review))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct FeedRow {
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
    pub remote_actor_url: Option<String>,
    pub user_email: String,
}

impl FeedRow {
    pub fn into_domain(self) -> Result<FeedEntry, DomainError> {
        let diary = DiaryRow {
            id: self.id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
            review_id: self.review_id,
            movie_id: self.movie_id,
            user_id: self.user_id,
            rating: self.rating,
            comment: self.comment,
            watched_at: self.watched_at,
            created_at: self.created_at,
            remote_actor_url: self.remote_actor_url,
        }
        .into_domain()?;
        Ok(FeedEntry::new(diary, self.user_email))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MovieStatsRow {
    pub total_count: i64,
    pub avg_rating: Option<f64>,
    pub federated_count: i64,
    pub rating_1: i64,
    pub rating_2: i64,
    pub rating_3: i64,
    pub rating_4: i64,
    pub rating_5: i64,
}

impl MovieStatsRow {
    pub fn into_domain(self) -> domain::models::MovieStats {
        domain::models::MovieStats {
            total_count: self.total_count as u64,
            avg_rating: self.avg_rating,
            federated_count: self.federated_count as u64,
            rating_histogram: [
                self.rating_1 as u64,
                self.rating_2 as u64,
                self.rating_3 as u64,
                self.rating_4 as u64,
                self.rating_5 as u64,
            ],
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct UserSummaryRow {
    pub id: String,
    pub email: String,
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub avatar_path: Option<String>,
}

impl UserSummaryRow {
    pub fn into_domain(self) -> Result<UserSummary, DomainError> {
        Ok(UserSummary::new(
            UserId::from_uuid(parse_uuid(&self.id)?),
            Email::new(self.email)?,
            self.total_movies,
            self.avg_rating,
            self.avatar_path,
        ))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct UserTotalsRow {
    pub total: i64,
    pub avg_rating: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct DirectorCountRow {
    pub director: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct MonthlyRatingRow {
    pub month: String,
    pub avg_rating: f64,
    pub count: i64,
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
