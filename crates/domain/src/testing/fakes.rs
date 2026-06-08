use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, FeedEntry, Movie, MovieStats, Review, ReviewHistory,
        collections::{PageParams, Paginated},
    },
    ports::{
        AuthService, DiaryRepository, FeedSortBy, FollowingFilter, GeneratedToken, MetadataClient,
        MetadataSearchCriteria, PasswordHasher,
    },
    value_objects::{ExternalMetadataId, MovieId, PasswordHash, PosterUrl, UserId},
};

// ── FakeAuthService ───────────────────────────────────────────────────────────

pub struct FakeAuthService;

#[async_trait]
impl AuthService for FakeAuthService {
    async fn generate_token(&self, user_id: &UserId) -> Result<GeneratedToken, DomainError> {
        Ok(GeneratedToken {
            token: user_id.value().to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        })
    }

    async fn validate_token(&self, token: &str) -> Result<UserId, DomainError> {
        Uuid::parse_str(token)
            .map(UserId::from_uuid)
            .map_err(|_| DomainError::Unauthorized("invalid token".into()))
    }
}

// ── FakePasswordHasher ────────────────────────────────────────────────────────

pub struct FakePasswordHasher;

#[async_trait]
impl PasswordHasher for FakePasswordHasher {
    async fn hash(&self, plain_password: &str) -> Result<PasswordHash, DomainError> {
        PasswordHash::new(format!("hashed:{plain_password}"))
    }

    async fn verify(&self, plain_password: &str, hash: &PasswordHash) -> Result<bool, DomainError> {
        Ok(hash.value() == format!("hashed:{plain_password}"))
    }
}

// ── FakeMetadataClient ────────────────────────────────────────────────────────

pub struct FakeMetadataClient;

#[async_trait]
impl MetadataClient for FakeMetadataClient {
    async fn fetch_movie_metadata(
        &self,
        _criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        Err(DomainError::InfrastructureError(
            "fake metadata client".into(),
        ))
    }

    async fn get_poster_url(
        &self,
        _external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        Ok(None)
    }
}

// ── FakeDiaryRepository ───────────────────────────────────────────────────────

pub struct FakeDiaryRepository {
    histories: Mutex<HashMap<Uuid, (Movie, Vec<Review>)>>,
}

impl FakeDiaryRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            histories: Mutex::new(HashMap::new()),
        })
    }

    pub fn seed_history(&self, movie: Movie, reviews: Vec<Review>) {
        self.histories
            .lock()
            .unwrap()
            .insert(movie.id().value(), (movie, reviews));
    }
}

#[async_trait]
impl DiaryRepository for FakeDiaryRepository {
    async fn query_diary(
        &self,
        _filter: &DiaryFilter,
    ) -> Result<Paginated<DiaryEntry>, DomainError> {
        unimplemented!("FakeDiaryRepository::query_diary")
    }

    async fn query_activity_feed(
        &self,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        unimplemented!("FakeDiaryRepository::query_activity_feed")
    }

    async fn query_activity_feed_filtered(
        &self,
        _page: &PageParams,
        _sort_by: &FeedSortBy,
        _search: Option<&str>,
        _following: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        unimplemented!("FakeDiaryRepository::query_activity_feed_filtered")
    }

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        let histories = self.histories.lock().unwrap();
        let (movie, reviews) = histories
            .get(&movie_id.value())
            .ok_or_else(|| DomainError::NotFound(format!("movie {}", movie_id.value())))?;
        Ok(ReviewHistory::new(movie.clone(), reviews.clone()))
    }

    async fn get_user_history(&self, _user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        unimplemented!("FakeDiaryRepository::get_user_history")
    }

    async fn get_movie_stats(&self, _movie_id: &MovieId) -> Result<MovieStats, DomainError> {
        unimplemented!("FakeDiaryRepository::get_movie_stats")
    }

    async fn get_movie_social_feed(
        &self,
        _movie_id: &MovieId,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        unimplemented!("FakeDiaryRepository::get_movie_social_feed")
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        unimplemented!("FakeDiaryRepository::count_local_posts")
    }
}
