use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use uuid::Uuid;

use crate::{
    errors::DomainError,
    events::{DomainEvent, EventEnvelope},
    models::wrapup::WrapUpReport,
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, EntityType, ExportFormat, ExternalPersonId,
        FeedEntry, FieldMapping, FileFormat, ImportError, ImportProfile, ImportSession,
        IndexableDocument, Movie, MovieFilter, MovieProfile, MovieStats, MovieSummary, ParsedFile,
        ParsedPlaybackEvent, Person, PersonCredits, PersonId, RemoteWatchlistEntry, Review,
        ReviewHistory, SearchQuery, SearchResults, User, UserStats, UserSummary, UserTrends,
        WatchEvent, WatchEventStatus, WatchlistEntry, WatchlistWithMovie, WebhookToken,
        collections::{self, PageParams, Paginated},
        wrapup::{DateRange, WrapUpRecord, WrapUpScope, WrapUpStatus},
    },
    value_objects::{
        Email, ExternalMetadataId, ImportProfileId, ImportSessionId, MovieId, MovieTitle,
        PasswordHash, PosterUrl, ReleaseYear, ReviewId, UserId, Username, WatchEventId,
        WebhookTokenId, WrapUpId,
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

impl std::str::FromStr for FeedSortBy {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "date_asc" => Self::DateAsc,
            "rating" => Self::Rating,
            "rating_asc" => Self::RatingAsc,
            _ => Self::Date,
        })
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

#[derive(Debug, Clone)]
pub struct PendingFollowerInfo {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[async_trait]
pub trait SocialQueryPort: Send + Sync {
    async fn get_accepted_following_urls(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<String>, DomainError>;

    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError>;

    async fn count_following(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn count_accepted_followers(&self, user_id: uuid::Uuid) -> Result<usize, DomainError>;

    async fn get_pending_followers(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<PendingFollowerInfo>, DomainError>;
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
    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError>;
    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError>;
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
pub trait ObjectStorage: Send + Sync {
    /// Stores `image_bytes` at `key` and returns the stored key.
    async fn store(&self, key: &str, image_bytes: &[u8]) -> Result<String, DomainError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, DomainError>;
    async fn get_stream(
        &self,
        key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>;
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
        profile: &crate::models::UserProfile,
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
    /// Insert a batch of missing persons from movie_cast/movie_crew into the persons table.
    /// Returns (inserted_count, has_more).
    async fn backfill_from_credits_batch(
        &self,
        batch_size: u32,
    ) -> Result<(u64, bool), DomainError>;
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
    async fn list_page(&self, limit: u32, offset: u32) -> Result<Vec<Person>, DomainError>;
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

/// Read-only query port used exclusively by the ActivityPub adapter.
/// Consolidates all reads the AP adapter needs so it never touches write repositories.
#[async_trait]
pub trait LocalApContentQuery: Send + Sync {
    async fn get_local_reviews_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DiaryEntry>, DomainError>;

    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError>;

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError>;

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError>;

    async fn count_local_posts(&self) -> Result<u64, DomainError>;

    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError>;

    async fn get_local_reviews_page(
        &self,
        user_id: &UserId,
        before: Option<chrono::NaiveDateTime>,
        limit: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
}

// ── Media server integration ──────────────────────────────────────────────────

pub trait MediaServerParser: Send + Sync {
    fn parse_playback_event(&self, body: &[u8])
    -> Result<Option<ParsedPlaybackEvent>, DomainError>;
}

#[async_trait]
pub trait WatchEventRepository: Send + Sync {
    async fn save(&self, event: &WatchEvent) -> Result<(), DomainError>;
    async fn update_status(
        &self,
        id: &WatchEventId,
        status: WatchEventStatus,
    ) -> Result<(), DomainError>;
    async fn list_pending(&self, user_id: &UserId) -> Result<Vec<WatchEvent>, DomainError>;
    async fn get_by_id(&self, id: &WatchEventId) -> Result<Option<WatchEvent>, DomainError>;
    async fn get_by_ids(&self, ids: &[WatchEventId]) -> Result<Vec<WatchEvent>, DomainError>;
    async fn update_status_batch(
        &self,
        ids: &[WatchEventId],
        status: WatchEventStatus,
    ) -> Result<u64, DomainError>;
    async fn find_duplicate(
        &self,
        user_id: &UserId,
        external_id: &str,
        after: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError>;
    async fn delete_non_pending_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError>;
}

#[async_trait]
pub trait WebhookTokenRepository: Send + Sync {
    async fn save(&self, token: &WebhookToken) -> Result<(), DomainError>;
    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<WebhookToken>, DomainError>;
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<WebhookToken>, DomainError>;
    async fn delete(&self, id: &WebhookTokenId, user_id: &UserId) -> Result<(), DomainError>;
    async fn touch_last_used(&self, id: &WebhookTokenId) -> Result<(), DomainError>;
}

#[async_trait]
pub trait WrapUpRepository: Send + Sync {
    async fn create(&self, record: &WrapUpRecord) -> Result<(), DomainError>;
    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn set_complete(&self, id: &WrapUpId, report: &WrapUpReport) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: &WrapUpId) -> Result<Option<WrapUpRecord>, DomainError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<WrapUpRecord>, DomainError>;
    async fn list_global(&self) -> Result<Vec<WrapUpRecord>, DomainError>;
    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<WrapUpRecord>, DomainError>;
    async fn delete(&self, id: &WrapUpId) -> Result<(), DomainError>;
    async fn delete_failed_older_than(
        &self,
        before: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError>;
}

// ── Wrap-up / Year-in-Review ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct WrapUpMovieRow {
    pub movie_id: Uuid,
    pub title: String,
    pub release_year: u16,
    pub director: Option<String>,
    pub poster_path: Option<String>,
    pub rating: u8,
    pub watched_at: NaiveDateTime,
    pub user_id: Uuid,
    pub runtime_minutes: Option<u32>,
    pub budget_usd: Option<i64>,
    pub original_language: Option<String>,
    pub genres: Vec<String>,
    pub keywords: Vec<String>,
    pub cast_names: Vec<(String, u32, i64)>,
    pub cast_profile_paths: Vec<Option<String>>,
}

#[async_trait]
pub trait WrapUpStatsQuery: Send + Sync {
    async fn get_reviews_with_profiles(
        &self,
        scope: &WrapUpScope,
        range: &DateRange,
    ) -> Result<Vec<WrapUpMovieRow>, DomainError>;
}

// ── Video renderer ──────────────────────────────────────────────────────────

pub struct VideoRenderAssets {
    pub poster_images: Vec<(String, Vec<u8>)>,
    pub cast_images: Vec<(String, Vec<u8>)>,
}

#[async_trait]
pub trait WrapUpVideoRenderer: Send + Sync {
    async fn render(
        &self,
        report: &WrapUpReport,
        assets: VideoRenderAssets,
    ) -> Result<Vec<u8>, DomainError>;
}
