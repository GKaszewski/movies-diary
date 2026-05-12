use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope},
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, ExportFormat, FeedEntry, FieldMapping,
        FileFormat, ImportError, ImportProfile, ImportSession, Movie, MovieProfile, MovieStats,
        ParsedFile, Review, ReviewHistory, User, UserStats, UserSummary, UserTrends,
        collections::{self, PageParams, Paginated},
    },
    value_objects::{
        Email, ExternalMetadataId, ImportProfileId, ImportSessionId, MovieId, MovieTitle,
        PasswordHash, PosterUrl, ReleaseYear, ReviewId, UserId, Username,
    },
};

pub trait DocumentParser: Send + Sync {
    fn parse(&self, bytes: &[u8], format: FileFormat) -> Result<ParsedFile, ImportError>;
    fn apply_mapping(&self, file: &ParsedFile, mappings: &[FieldMapping]) -> Vec<AnnotatedRow>;
}

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
    async fn list_movies(
        &self,
        page: &collections::PageParams,
        search: Option<&str>,
    ) -> Result<collections::Paginated<Movie>, DomainError>;
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
    async fn get_movie_stats(&self, movie_id: &MovieId) -> Result<MovieStats, DomainError>;
    async fn get_movie_social_feed(
        &self,
        movie_id: &MovieId,
        page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError>;
    async fn count_local_posts(&self) -> Result<u64, DomainError>;
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
pub trait ImageStorage: Send + Sync {
    /// Stores `image_bytes` at `key` and returns the stored key.
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError>;
    async fn delete(&self, key: &str) -> Result<(), DomainError>;
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
    async fn update_profile(
        &self,
        user_id: &UserId,
        bio: Option<String>,
        avatar_path: Option<String>,
    ) -> Result<(), DomainError>;
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

#[async_trait]
pub trait PeriodicJob: Send + Sync {
    fn interval(&self) -> std::time::Duration;
    async fn run(&self) -> Result<(), DomainError>;
}

#[async_trait]
pub trait MovieProfileRepository: Send + Sync {
    async fn upsert(&self, profile: &MovieProfile) -> Result<(), DomainError>;
    async fn get_by_movie_id(&self, id: &MovieId) -> Result<Option<MovieProfile>, DomainError>;
    /// Returns (movie_id, external_metadata_id) for movies with no profile or a stale one
    /// (enriched_at older than 30 days).
    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError>;
}

#[async_trait]
pub trait MovieEnrichmentClient: Send + Sync {
    /// Resolves an external ID (TMDb or IMDb) and fetches the full movie profile.
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError>;
}

#[async_trait]
pub trait ImportSessionRepository: Send + Sync {
    async fn create(&self, session: &ImportSession) -> Result<(), DomainError>;
    async fn get(&self, id: &ImportSessionId, user_id: &UserId) -> Result<Option<ImportSession>, DomainError>;
    async fn update(&self, session: &ImportSession) -> Result<(), DomainError>;
    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError>;
    async fn delete_expired(&self) -> Result<u64, DomainError>;
    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImportProfileRepository: Send + Sync {
    async fn save(&self, profile: &ImportProfile) -> Result<(), DomainError>;
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError>;
    async fn get(&self, id: &ImportProfileId, user_id: &UserId) -> Result<Option<ImportProfile>, DomainError>;
    async fn delete(&self, id: &ImportProfileId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImageRefCommand: Send + Sync {
    async fn swap(&self, old_key: &str, new_key: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImageRefQuery: Send + Sync {
    async fn list_keys(&self) -> Result<Vec<String>, DomainError>;
}
