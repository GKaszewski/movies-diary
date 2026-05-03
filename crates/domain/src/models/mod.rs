use chrono::{NaiveDateTime, Utc};

use crate::{
    errors::DomainError,
    models::collections::PageParams,
    value_objects::{
        Comment, Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, Rating,
        ReleaseYear, ReviewId, UserId,
    },
};
pub mod collections;

#[derive(Clone, Debug, Default)]
pub enum SortDirection {
    #[default]
    Descending,
    Ascending,
}

#[derive(Clone, Debug, Default)]
pub struct DiaryFilter {
    pub sort_by: SortDirection,
    pub page: PageParams,
    pub movie_id: Option<MovieId>,
}

#[derive(Clone, Debug)]
pub struct Movie {
    id: MovieId,
    external_metadata_id: Option<ExternalMetadataId>,
    title: MovieTitle,
    release_year: ReleaseYear,
    director: Option<String>,
    poster_path: Option<PosterPath>,
}

impl Movie {
    pub fn new(
        external_metadata_id: Option<ExternalMetadataId>,
        title: MovieTitle,
        release_year: ReleaseYear,
        director: Option<String>,
        poster_path: Option<PosterPath>,
    ) -> Self {
        Self {
            id: MovieId::generate(),
            external_metadata_id,
            title,
            release_year,
            director,
            poster_path,
        }
    }

    pub fn update_poster(&mut self, poster_path: PosterPath) {
        self.poster_path = Some(poster_path);
    }

    pub fn id(&self) -> &MovieId {
        &self.id
    }
    pub fn external_metadata_id(&self) -> Option<&ExternalMetadataId> {
        self.external_metadata_id.as_ref()
    }
    pub fn title(&self) -> &MovieTitle {
        &self.title
    }
    pub fn release_year(&self) -> &ReleaseYear {
        &self.release_year
    }
    pub fn director(&self) -> Option<&str> {
        self.director.as_deref()
    }
    pub fn poster_path(&self) -> Option<&PosterPath> {
        self.poster_path.as_ref()
    }
}

impl Movie {
    pub fn is_manual_match(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
        director: Option<&str>,
    ) -> bool {
        if self.title != *title || self.release_year != *year {
            return false;
        }

        match (self.director(), director) {
            (Some(existing_dir), Some(new_dir)) => existing_dir.eq_ignore_ascii_case(new_dir),
            _ => true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Review {
    id: ReviewId,
    movie_id: MovieId,
    user_id: UserId,
    rating: Rating,
    comment: Option<Comment>,
    watched_at: chrono::NaiveDateTime,
    created_at: chrono::NaiveDateTime,
}

impl Review {
    pub fn new(
        movie_id: MovieId,
        user_id: UserId,
        rating: Rating,
        comment: Option<Comment>,
        watched_at: NaiveDateTime,
    ) -> Result<Self, DomainError> {
        let now = Utc::now().naive_utc();

        if watched_at > now {
            return Err(DomainError::ValidationError(
                "watched_at cannot be in the future".into(),
            ));
        }

        Ok(Self {
            id: ReviewId::generate(),
            movie_id,
            user_id,
            rating,
            comment,
            watched_at,
            created_at: now,
        })
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
    pub fn viewings_mut(&mut self) -> &mut Vec<Review> {
        &mut self.viewings
    }
}

#[derive(Clone, Debug)]
pub struct User {
    id: UserId,
    email: Email,
    password_hash: PasswordHash,
}

impl User {
    pub fn new(email: Email, password_hash: PasswordHash) -> Self {
        Self {
            id: UserId::generate(),
            email,
            password_hash,
        }
    }

    pub fn update_password(&mut self, new_hash: PasswordHash) {
        self.password_hash = new_hash;
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }
}
