use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, Movie, PersistedReview, Review, ReviewSource, WatchlistEntry,
        WatchlistWithMovie,
    },
    value_objects::{
        Comment, ExternalMetadataId, MovieId, MovieTitle, PosterPath, Rating, ReleaseYear,
        ReviewId, UserId, WatchlistEntryId,
    },
};

use crate::shared::{parse_datetime, parse_uuid};

#[derive(sqlx::FromRow)]
pub(super) struct MovieRow {
    id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
}

impl MovieRow {
    pub(super) fn into_domain(self) -> Result<Movie, DomainError> {
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
pub(super) struct ReviewRow {
    id: String,
    movie_id: String,
    user_id: String,
    rating: i64,
    comment: Option<String>,
    watched_at: String,
    created_at: String,
    remote_actor_url: Option<String>,
    watch_medium: Option<String>,
    spoilers: bool,
}

impl ReviewRow {
    pub(super) fn into_domain(self) -> Result<Review, DomainError> {
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
        let watch_medium = self.watch_medium.map(|s| s.parse()).transpose()?;
        Ok(Review::from_persistence(PersistedReview {
            id,
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at,
            source,
            watch_medium,
            spoilers: self.spoilers,
        }))
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct DiaryRow {
    id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
    review_id: String,
    movie_id: String,
    user_id: String,
    rating: i64,
    comment: Option<String>,
    watched_at: String,
    created_at: String,
    remote_actor_url: Option<String>,
    watch_medium: Option<String>,
    spoilers: bool,
}

impl DiaryRow {
    pub(super) fn into_domain(self) -> Result<DiaryEntry, DomainError> {
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
            watch_medium: self.watch_medium,
            spoilers: self.spoilers,
        }
        .into_domain()?;
        Ok(DiaryEntry::new(movie, review))
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct WatchlistRow {
    id: String,
    user_id: String,
    movie_id: String,
    added_at: String,
    m_id: String,
    external_metadata_id: Option<String>,
    title: String,
    release_year: i64,
    director: Option<String>,
    poster_path: Option<String>,
}

impl WatchlistRow {
    pub(super) fn into_domain(self) -> Result<WatchlistWithMovie, DomainError> {
        let entry = WatchlistEntry {
            id: WatchlistEntryId::from_uuid(parse_uuid(&self.id)?),
            user_id: UserId::from_uuid(parse_uuid(&self.user_id)?),
            movie_id: MovieId::from_uuid(parse_uuid(&self.movie_id)?),
            added_at: parse_datetime(&self.added_at)?,
        };
        let movie = MovieRow {
            id: self.m_id,
            external_metadata_id: self.external_metadata_id,
            title: self.title,
            release_year: self.release_year,
            director: self.director,
            poster_path: self.poster_path,
        }
        .into_domain()?;
        Ok(WatchlistWithMovie { entry, movie })
    }
}

// ── LocalApContentQuery impl ─────────────────────────────────────────────────
