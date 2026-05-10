use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope},
    models::{
        DiaryEntry, DiaryFilter, ExportFormat, FeedEntry, Movie, Review, ReviewHistory, User,
        UserStats, UserSummary, UserTrends,
        collections::{PageParams, Paginated},
    },
    value_objects::{
        Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, PosterUrl,
        ReleaseYear, ReviewId, UserId, Username,
    },
};

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FeedSortBy {
    #[default]
    Date,
    DateAsc,
    Rating,
    RatingAsc,
}

impl FeedSortBy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "date_asc" => Self::DateAsc,
            "rating" => Self::Rating,
            "rating_asc" => Self::RatingAsc,
            _ => Self::Date,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FollowingFilter {
    pub local_user_ids: Vec<uuid::Uuid>,
    pub remote_actor_urls: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteActorInfo {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
}

/// New trait for social/federation read queries
#[async_trait]
pub trait SocialQueryPort: Send + Sync {
    /// Returns all accepted remote_actor_urls followed by `user_id`.
    async fn get_accepted_following_urls(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<String>, DomainError>;

    /// Returns all distinct remote actors followed by any local user on this instance.
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<RemoteActorInfo>, DomainError>;
}

#[async_trait]
pub trait MovieRepository: Send + Sync {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError>;
    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError>;
    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError>;
    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError>;
    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError>;
    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError>;
    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError>;
    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError>;
}

#[async_trait]
pub trait DiaryRepository: Send + Sync {
    async fn query_diary(&self, filter: &DiaryFilter)
    -> Result<Paginated<DiaryEntry>, DomainError>;
    async fn query_activity_feed(
        &self,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn query_activity_feed_filtered(
        &self,
        page: &PageParams,
        sort_by: &FeedSortBy,
        search: Option<&str>,
        following: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError>;
    async fn get_user_history(&self, user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError>;
}

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn get_user_stats(&self, user_id: &UserId) -> Result<UserStats, DomainError>;
    async fn get_user_trends(&self, user_id: &UserId) -> Result<UserTrends, DomainError>;
}

pub enum MetadataSearchCriteria {
    ImdbId(ExternalMetadataId),
    Title {
        title: MovieTitle,
        year: Option<ReleaseYear>,
    },
}

#[async_trait]
pub trait MetadataClient: Send + Sync {
    async fn fetch_movie_metadata(
        &self,
        criteria: &MetadataSearchCriteria,
    ) -> Result<Movie, DomainError>;
    async fn get_poster_url(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError>;
}

#[async_trait]
pub trait PosterFetcherClient: Send + Sync {
    async fn fetch_poster_bytes(&self, poster_url: &PosterUrl) -> Result<Vec<u8>, DomainError>;
}

#[async_trait]
pub trait PosterStorage: Send + Sync {
    async fn store_poster(
        &self,
        movie_id: &MovieId,
        image_bytes: &[u8],
    ) -> Result<PosterPath, DomainError>;

    async fn get_poster(&self, poster_path: &PosterPath) -> Result<Vec<u8>, DomainError>;
}

pub struct GeneratedToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn generate_token(&self, user_id: &UserId) -> Result<GeneratedToken, DomainError>;
    async fn validate_token(&self, token: &str) -> Result<UserId, DomainError>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError>;
    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError>;
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

pub trait EventConsumer: Send + Sync {
    /// Returns a stream of event envelopes. Each envelope carries a domain event
    /// and an ack handle — callers ack after successful dispatch, nack on failure.
    /// Implementations decide transport (NATS, DB queue, in-memory channel).
    fn consume(&self) -> futures::stream::BoxStream<'_, Result<EventEnvelope, DomainError>>;
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plain_password: &str) -> Result<PasswordHash, DomainError>;

    async fn verify(&self, plain_password: &str, hash: &PasswordHash) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait DiaryExporter: Send + Sync {
    async fn serialize_entries(
        &self,
        entries: &[DiaryEntry],
        format: ExportFormat,
    ) -> Result<Vec<u8>, DomainError>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError>;
}
