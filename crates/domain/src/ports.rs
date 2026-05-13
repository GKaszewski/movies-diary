use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope},
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, EntityType, ExportFormat, ExternalPersonId,
        FeedEntry, FieldMapping, FileFormat, ImportError, ImportProfile, ImportSession,
        IndexableDocument, Movie, MovieFilter, MovieProfile, MovieStats, MovieSummary, ParsedFile,
        Person, PersonCredits, PersonId, RemoteWatchlistEntry, Review, ReviewHistory, SearchQuery,
        SearchResults, User, UserStats, UserSummary, UserTrends, WatchlistEntry,
        WatchlistWithMovie,
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
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError>;
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
        filter: &MovieFilter,
    ) -> Result<collections::Paginated<MovieSummary>, DomainError>;
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
        banner_path: Option<String>,
        also_known_as: Option<String>,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait UserProfileFieldsRepository: Send + Sync {
    async fn get_fields(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<crate::models::ProfileField>, DomainError>;
    async fn set_fields(
        &self,
        user_id: &UserId,
        fields: Vec<crate::models::ProfileField>,
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
    async fn get(
        &self,
        id: &ImportSessionId,
        user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError>;
    async fn update(&self, session: &ImportSession) -> Result<(), DomainError>;
    async fn delete(&self, id: &ImportSessionId) -> Result<(), DomainError>;
    async fn delete_expired(&self) -> Result<u64, DomainError>;
    async fn delete_expired_for_user(&self, user_id: &UserId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ImportProfileRepository: Send + Sync {
    async fn save(&self, profile: &ImportProfile) -> Result<(), DomainError>;
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError>;
    async fn get(
        &self,
        id: &ImportProfileId,
        user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError>;
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

/// Write port — mutates the persons table. No reads.
#[async_trait]
pub trait PersonCommand: Send + Sync {
    /// Upsert a batch of persons. Uses INSERT OR REPLACE (SQLite) / ON CONFLICT DO UPDATE (Postgres).
    async fn upsert_batch(&self, persons: &[Person]) -> Result<(), DomainError>;
}

/// Read port — queries persons and credits. No mutations.
#[async_trait]
pub trait PersonQuery: Send + Sync {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError>;
    async fn get_by_external_id(
        &self,
        id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError>;
    /// Returns the person's full cast and crew credit history across all indexed movies.
    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError>;
    /// Returns persons who have no remaining entries in movie_cast or movie_crew.
    /// Called after movie deletion to find index entries that can be pruned.
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError>;
}

/// Read port — executes search queries. No mutations.
#[async_trait]
pub trait SearchPort: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, DomainError>;
}

/// Write port — manages the search index. No reads.
#[async_trait]
pub trait SearchCommand: Send + Sync {
    /// Add or replace a document in the search index.
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError>;
    /// Remove a document from the search index by entity type and internal ID string.
    async fn remove(&self, entity_type: EntityType, id: &str) -> Result<(), DomainError>;
}

#[async_trait]
pub trait WatchlistRepository: Send + Sync {
    /// Add a new entry. Silently succeeds if the entry already exists.
    async fn add(&self, entry: &WatchlistEntry) -> Result<(), DomainError>;

    /// Remove an entry. Returns NotFound if the entry does not exist.
    async fn remove(&self, user_id: &UserId, movie_id: &MovieId) -> Result<(), DomainError>;

    /// Remove an entry if it exists. Never returns NotFound.
    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
    ) -> Result<bool, DomainError>;

    async fn get_for_user(
        &self,
        user_id: &UserId,
        page: &collections::PageParams,
    ) -> Result<collections::Paginated<WatchlistWithMovie>, DomainError>;

    async fn contains(&self, user_id: &UserId, movie_id: &MovieId) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait RemoteWatchlistRepository: Send + Sync {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), DomainError>;
    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError>;
    /// Find entries for a remote actor whose URL hashes (v5 UUID) to the given UUID.
    async fn get_by_derived_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
}
