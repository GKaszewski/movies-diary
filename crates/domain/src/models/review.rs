use chrono::{NaiveDateTime, Utc};

use crate::{
    errors::DomainError,
    value_objects::{Comment, MovieId, Rating, ReviewId, UserId},
};

use super::movie::Movie;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ReviewSource {
    #[default]
    Local,
    Remote {
        actor_url: String,
    },
}

pub struct PersistedReview {
    pub id: ReviewId,
    pub movie_id: MovieId,
    pub user_id: UserId,
    pub rating: Rating,
    pub comment: Option<Comment>,
    pub watched_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub source: ReviewSource,
}

#[derive(Clone, Debug)]
pub struct Review {
    id: ReviewId,
    movie_id: MovieId,
    user_id: UserId,
    rating: Rating,
    comment: Option<Comment>,
    watched_at: NaiveDateTime,
    created_at: NaiveDateTime,
    source: ReviewSource,
}

impl Review {
    pub fn new(
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        comment: Option<Comment>,
        watched_at: NaiveDateTime,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: ReviewId::generate(),
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at: Utc::now().naive_utc(),
            source: ReviewSource::Local,
        })
    }

    pub fn from_persistence(row: PersistedReview) -> Self {
        Self {
            id: row.id,
            movie_id: row.movie_id,
            user_id: row.user_id,
            rating: row.rating,
            comment: row.comment,
            watched_at: row.watched_at,
            created_at: row.created_at,
            source: row.source,
        }
    }

    pub fn id(&self) -> &ReviewId {
        &self.id
    }
    pub fn movie_id(&self) -> &MovieId {
        &self.movie_id
    }
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
    pub fn rating(&self) -> &Rating {
        &self.rating
    }
    pub fn comment(&self) -> Option<&Comment> {
        self.comment.as_ref()
    }
    pub fn watched_at(&self) -> &NaiveDateTime {
        &self.watched_at
    }
    pub fn created_at(&self) -> &NaiveDateTime {
        &self.created_at
    }
    pub fn source(&self) -> &ReviewSource {
        &self.source
    }
    /// Returns [star1_filled, star2_filled, ..., star5_filled]
    pub fn stars(&self) -> [bool; 5] {
        let r = self.rating.value();
        [r >= 1, r >= 2, r >= 3, r >= 4, r >= 5]
    }

    pub fn is_remote(&self) -> bool {
        matches!(self.source, ReviewSource::Remote { .. })
    }
}

#[derive(Clone, Debug)]
pub struct DiaryEntry {
    movie: Movie,
    review: Review,
}

impl DiaryEntry {
    pub fn new(movie: Movie, review: Review) -> Self {
        Self { movie, review }
    }

    pub fn movie(&self) -> &Movie {
        &self.movie
    }
    pub fn review(&self) -> &Review {
        &self.review
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiaryFilter {
    pub sort_by: super::SortDirection,
    pub page: crate::models::collections::PageParams,
    pub movie_id: Option<MovieId>,
    pub user_id: Option<UserId>,
    pub search: Option<String>,
    pub include_remote: bool,
}

#[derive(Clone, Debug)]
pub struct ReviewHistory {
    movie: Movie,
    viewings: Vec<Review>,
}

impl ReviewHistory {
    pub fn new(movie: Movie, viewings: Vec<Review>) -> Self {
        Self { movie, viewings }
    }

    pub fn movie(&self) -> &Movie {
        &self.movie
    }
    pub fn viewings(&self) -> &[Review] {
        &self.viewings
    }
    pub fn sort_by_date(&mut self) {
        self.viewings.sort_by_key(|r| *r.watched_at());
    }
}
