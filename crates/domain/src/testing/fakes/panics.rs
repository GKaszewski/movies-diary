use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, EntityType, ExportFormat, ExternalPersonId,
        FeedEntry, FeedSortBy, FieldMapping, FileFormat, FollowingFilter, ImportError,
        ImportProfile, ImportSession, IndexableDocument, MovieProfile, MovieStats, ParsedFile,
        Person, PersonCredits, PersonEnrichmentData, PersonId, RefreshSession, RemoteActorInfo,
        ReviewHistory, SearchQuery, SearchResults, UserStats, UserTrends, WatchlistEntry,
        WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::{
        DiaryExporter, DiaryQuery, DocumentParser, ImportProfileRepository,
        ImportSessionRepository, MovieProfileRepository, PersonCommand, PersonQuery,
        PosterFetcherClient, RefreshSessionRepository, SearchCommand, SearchPort, StatsRepository,
        UserProfileFieldsRepository, WatchlistRepository,
    },
    value_objects::{ImportProfileId, ImportSessionId, MovieId, PosterUrl, UserId},
};

use crate::{
    events::DomainEvent,
    models::{GeneratedToken, Movie, Review},
    ports::{
        AuthService, EventPublisher, MetadataClient, MovieCommand, MovieQuery, ObjectStorage,
        PasswordHasher, ReviewRepository, UserRepository,
    },
    value_objects::{Email, ExternalMetadataId, MovieTitle, PasswordHash, ReleaseYear, ReviewId},
};

// ── PanicDiaryQuery ───────────────────────────────────────────────────────────

pub struct PanicDiaryQuery;

#[async_trait]
impl DiaryQuery for PanicDiaryQuery {
    async fn query_diary(&self, _: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn query_activity_feed(
        &self,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn query_activity_feed_filtered(
        &self,
        _: &PageParams,
        _: &FeedSortBy,
        _: Option<&str>,
        _: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn get_review_history(&self, _: &MovieId) -> Result<ReviewHistory, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn get_user_history(&self, _: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    fn stream_user_history(
        &self,
        _: UserId,
    ) -> futures::stream::BoxStream<'static, Result<DiaryEntry, DomainError>> {
        panic!("PanicDiaryQuery called")
    }
    async fn get_movie_stats(&self, _: &MovieId) -> Result<MovieStats, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn get_movie_social_feed(
        &self,
        _: &MovieId,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryQuery called")
    }
    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        panic!("PanicDiaryQuery called")
    }
}

pub struct PanicStatsRepository;

#[async_trait]
impl StatsRepository for PanicStatsRepository {
    async fn get_user_stats(&self, _: &UserId) -> Result<UserStats, DomainError> {
        panic!("PanicStatsRepository called")
    }
    async fn get_user_trends(&self, _: &UserId) -> Result<UserTrends, DomainError> {
        panic!("PanicStatsRepository called")
    }
    async fn count_reviews_in_year(&self, _: &UserId, _: u16) -> Result<u32, DomainError> {
        panic!("PanicStatsRepository called")
    }
}

pub struct PanicImportSessionRepository;

#[async_trait]
impl ImportSessionRepository for PanicImportSessionRepository {
    async fn create(&self, _: &ImportSession) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }
    async fn get(
        &self,
        _: &ImportSessionId,
        _: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        panic!("PanicImportSessionRepository called")
    }
    async fn update(&self, _: &ImportSession) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }
    async fn delete(&self, _: &ImportSessionId) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }
    async fn delete_expired(&self) -> Result<u64, DomainError> {
        panic!("PanicImportSessionRepository called")
    }
    async fn delete_expired_for_user(&self, _: &UserId) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }
}

pub struct PanicRefreshSessionRepository;

#[async_trait]
impl RefreshSessionRepository for PanicRefreshSessionRepository {
    async fn create(&self, _: &RefreshSession) -> Result<(), DomainError> {
        panic!("PanicRefreshSessionRepository called")
    }
    async fn get_by_token(&self, _: &str) -> Result<Option<RefreshSession>, DomainError> {
        panic!("PanicRefreshSessionRepository called")
    }
    async fn revoke(&self, _: &str) -> Result<(), DomainError> {
        panic!("PanicRefreshSessionRepository called")
    }
    async fn revoke_all_for_user(&self, _: &UserId) -> Result<(), DomainError> {
        panic!("PanicRefreshSessionRepository called")
    }
    async fn delete_expired(&self) -> Result<u64, DomainError> {
        panic!("PanicRefreshSessionRepository called")
    }
}

pub struct PanicImportProfileRepository;

#[async_trait]
impl ImportProfileRepository for PanicImportProfileRepository {
    async fn save(&self, _: &ImportProfile) -> Result<(), DomainError> {
        panic!("PanicImportProfileRepository called")
    }
    async fn list_for_user(&self, _: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        panic!("PanicImportProfileRepository called")
    }
    async fn get(
        &self,
        _: &ImportProfileId,
        _: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError> {
        panic!("PanicImportProfileRepository called")
    }
    async fn delete(&self, _: &ImportProfileId) -> Result<(), DomainError> {
        panic!("PanicImportProfileRepository called")
    }
}

pub struct PanicMovieProfileRepository;

#[async_trait]
impl MovieProfileRepository for PanicMovieProfileRepository {
    async fn upsert(&self, _: &MovieProfile) -> Result<(), DomainError> {
        panic!("PanicMovieProfileRepository called")
    }
    async fn get_by_movie_id(&self, _: &MovieId) -> Result<Option<MovieProfile>, DomainError> {
        panic!("PanicMovieProfileRepository called")
    }
    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError> {
        panic!("PanicMovieProfileRepository called")
    }
}

pub struct PanicPersonCommand;

#[async_trait]
impl PersonCommand for PanicPersonCommand {
    async fn upsert_batch(&self, _: &[Person]) -> Result<(), DomainError> {
        panic!("PanicPersonCommand called")
    }
    async fn backfill_from_credits_batch(&self, _: u32) -> Result<(u64, bool), DomainError> {
        panic!("PanicPersonCommand called")
    }
    async fn update_enrichment(
        &self,
        _: &PersonId,
        _: &PersonEnrichmentData,
    ) -> Result<(), DomainError> {
        panic!("PanicPersonCommand called")
    }
}

pub struct PanicPersonQuery;

#[async_trait]
impl PersonQuery for PanicPersonQuery {
    async fn get_by_id(&self, _: &PersonId) -> Result<Option<Person>, DomainError> {
        panic!("PanicPersonQuery called")
    }
    async fn get_by_external_id(
        &self,
        _: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        panic!("PanicPersonQuery called")
    }
    async fn get_credits(&self, _: &PersonId) -> Result<PersonCredits, DomainError> {
        panic!("PanicPersonQuery called")
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        panic!("PanicPersonQuery called")
    }
    async fn list_page(&self, _: u32, _: u32) -> Result<Vec<Person>, DomainError> {
        panic!("PanicPersonQuery called")
    }
}

pub struct PanicSearchPort;

#[async_trait]
impl SearchPort for PanicSearchPort {
    async fn search(&self, _: &SearchQuery) -> Result<SearchResults, DomainError> {
        Ok(SearchResults {
            movies: Paginated {
                items: vec![],
                total_count: 0,
                limit: 10,
                offset: 0,
            },
            people: Paginated {
                items: vec![],
                total_count: 0,
                limit: 10,
                offset: 0,
            },
        })
    }
}

pub struct PanicSearchCommand;

#[async_trait]
impl SearchCommand for PanicSearchCommand {
    async fn index(&self, _: IndexableDocument) -> Result<(), DomainError> {
        panic!("PanicSearchCommand called")
    }
    async fn remove(&self, _: EntityType, _: &str) -> Result<(), DomainError> {
        panic!("PanicSearchCommand called")
    }
}

pub struct PanicPosterFetcher;

#[async_trait]
impl PosterFetcherClient for PanicPosterFetcher {
    async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        panic!("PanicPosterFetcher called")
    }
}

pub struct PanicDiaryExporter;

impl DiaryExporter for PanicDiaryExporter {
    fn stream_entries(
        &self,
        _stream: futures::stream::BoxStream<'static, Result<DiaryEntry, DomainError>>,
        _format: ExportFormat,
    ) -> futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>> {
        panic!("PanicDiaryExporter called")
    }
}

pub struct PanicDocumentParser;

impl DocumentParser for PanicDocumentParser {
    fn parse(&self, _: &[u8], _: FileFormat) -> Result<ParsedFile, ImportError> {
        panic!("PanicDocumentParser called")
    }
    fn apply_mapping(&self, _: &ParsedFile, _: &[FieldMapping]) -> Vec<AnnotatedRow> {
        panic!("PanicDocumentParser called")
    }
}

pub struct PanicWatchlistRepository;

#[async_trait]
impl WatchlistRepository for PanicWatchlistRepository {
    async fn add(&self, _: &WatchlistEntry, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicWatchlistRepository called")
    }
    async fn remove(&self, _: &UserId, _: &MovieId, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicWatchlistRepository called")
    }
    async fn remove_if_present(
        &self,
        _: &UserId,
        _: &MovieId,
        _: &[DomainEvent],
    ) -> Result<bool, DomainError> {
        panic!("PanicWatchlistRepository called")
    }
    async fn get_for_user(
        &self,
        _: &UserId,
        _: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        panic!("PanicWatchlistRepository called")
    }
    async fn contains(&self, _: &UserId, _: &MovieId) -> Result<bool, DomainError> {
        panic!("PanicWatchlistRepository called")
    }
}

pub struct PanicRemoteWatchlistRepository;

#[async_trait]
impl crate::ports::RemoteWatchlistRepository for PanicRemoteWatchlistRepository {
    async fn save(&self, _: crate::models::RemoteWatchlistEntry) -> Result<(), DomainError> {
        panic!("PanicRemoteWatchlistRepository called")
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        panic!("PanicRemoteWatchlistRepository called")
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        panic!("PanicRemoteWatchlistRepository called")
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        panic!("PanicRemoteWatchlistRepository called")
    }
    async fn get_by_derived_uuid(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        panic!("PanicRemoteWatchlistRepository called")
    }
}

pub struct PanicProfileFieldsRepo;

#[async_trait]
impl UserProfileFieldsRepository for PanicProfileFieldsRepo {
    async fn get_fields(
        &self,
        _: &UserId,
    ) -> Result<Vec<crate::models::ProfileField>, DomainError> {
        panic!("PanicProfileFieldsRepo called")
    }
    async fn set_fields(
        &self,
        _: &UserId,
        _: Vec<crate::models::ProfileField>,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        panic!("PanicProfileFieldsRepo called")
    }
}

pub struct PanicFederationAdminQuery;

#[async_trait]
impl crate::ports::FederationAdminQuery for PanicFederationAdminQuery {
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError> {
        panic!("PanicFederationAdminQuery called")
    }
}

pub struct PanicFederatedProfileQuery;

#[async_trait]
impl crate::ports::FederatedProfileQuery for PanicFederatedProfileQuery {
    async fn get_federated_profile(
        &self,
        _: uuid::Uuid,
    ) -> Result<Option<crate::models::FederatedProfile>, DomainError> {
        panic!("PanicFederatedProfileQuery called")
    }
}

pub struct PanicWatchEventCommand;

#[async_trait]
impl crate::ports::WatchEventCommand for PanicWatchEventCommand {
    async fn save(
        &self,
        _: &crate::models::WatchEvent,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        panic!("PanicWatchEventCommand called")
    }
    async fn update_status(
        &self,
        _: &crate::value_objects::WatchEventId,
        _: crate::models::WatchEventStatus,
    ) -> Result<(), DomainError> {
        panic!("PanicWatchEventCommand called")
    }
    async fn update_status_batch(
        &self,
        _: &[crate::value_objects::WatchEventId],
        _: crate::models::WatchEventStatus,
    ) -> Result<u64, DomainError> {
        panic!("PanicWatchEventCommand called")
    }
    async fn delete_non_pending_older_than(
        &self,
        _: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        panic!("PanicWatchEventCommand called")
    }
}

pub struct PanicWatchEventQuery;

#[async_trait]
impl crate::ports::WatchEventQuery for PanicWatchEventQuery {
    async fn list_pending(
        &self,
        _: &UserId,
    ) -> Result<Vec<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventQuery called")
    }
    async fn get_by_id(
        &self,
        _: &crate::value_objects::WatchEventId,
    ) -> Result<Option<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventQuery called")
    }
    async fn get_by_ids(
        &self,
        _: &[crate::value_objects::WatchEventId],
    ) -> Result<Vec<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventQuery called")
    }
    async fn find_duplicate(
        &self,
        _: &UserId,
        _: &str,
        _: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        panic!("PanicWatchEventQuery called")
    }
}

pub struct PanicWebhookTokenRepository;

#[async_trait]
impl crate::ports::WebhookTokenRepository for PanicWebhookTokenRepository {
    async fn save(&self, _: &crate::models::WebhookToken) -> Result<(), DomainError> {
        panic!("PanicWebhookTokenRepository called")
    }
    async fn find_by_token_hash(
        &self,
        _: &str,
    ) -> Result<Option<crate::models::WebhookToken>, DomainError> {
        panic!("PanicWebhookTokenRepository called")
    }
    async fn list_by_user(
        &self,
        _: &UserId,
    ) -> Result<Vec<crate::models::WebhookToken>, DomainError> {
        panic!("PanicWebhookTokenRepository called")
    }
    async fn delete(
        &self,
        _: &crate::value_objects::WebhookTokenId,
        _: &UserId,
    ) -> Result<(), DomainError> {
        panic!("PanicWebhookTokenRepository called")
    }
    async fn touch_last_used(
        &self,
        _: &crate::value_objects::WebhookTokenId,
    ) -> Result<(), DomainError> {
        panic!("PanicWebhookTokenRepository called")
    }
}
pub struct PanicMovieCommand;

#[async_trait]
impl MovieCommand for PanicMovieCommand {
    async fn upsert_movie(&self, _: &Movie, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicMovieCommand called")
    }
    async fn delete_movie(&self, _: &MovieId, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicMovieCommand called")
    }
}

pub struct PanicMovieQuery;

#[async_trait]
impl MovieQuery for PanicMovieQuery {
    async fn get_movie_by_external_id(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        panic!("PanicMovieQuery called")
    }
    async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> {
        panic!("PanicMovieQuery called")
    }
    async fn get_movies_by_title_and_year(
        &self,
        _: &MovieTitle,
        _: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        panic!("PanicMovieQuery called")
    }
    async fn existing_external_ids(
        &self,
        _: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        panic!("PanicMovieQuery called")
    }
    async fn existing_title_year_pairs(
        &self,
        _: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        panic!("PanicMovieQuery called")
    }
    async fn list_movies(
        &self,
        _: &crate::models::collections::PageParams,
        _: &crate::models::MovieFilter,
    ) -> Result<crate::models::collections::Paginated<crate::models::MovieSummary>, DomainError>
    {
        panic!("PanicMovieQuery called")
    }
    async fn list_movies_with_external_id(&self) -> Result<Vec<crate::models::Movie>, DomainError> {
        Ok(vec![])
    }
}

pub struct PanicReviewRepository;

#[async_trait]
impl ReviewRepository for PanicReviewRepository {
    async fn save_review(&self, _: &Review, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicReviewRepository called")
    }
    async fn get_review_by_id(&self, _: &ReviewId) -> Result<Option<Review>, DomainError> {
        panic!("PanicReviewRepository called")
    }
    async fn update_review(&self, _: &Review, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicReviewRepository called")
    }
    async fn delete_review_and_orphaned_movie(
        &self,
        _: &ReviewId,
        _: &MovieId,
        _: &[DomainEvent],
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        panic!("PanicReviewRepository called")
    }
    async fn get_all_reviews_for_user(&self, _: &UserId) -> Result<Vec<Review>, DomainError> {
        panic!("PanicReviewRepository called")
    }
}

pub struct PanicMetadataClient;

#[async_trait]
impl MetadataClient for PanicMetadataClient {
    async fn fetch_movie_metadata(
        &self,
        _: &crate::models::MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        panic!("PanicMetadataClient called")
    }
    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        panic!("PanicMetadataClient called")
    }
}

pub struct PanicObjectStorage;

#[async_trait]
impl ObjectStorage for PanicObjectStorage {
    async fn store(&self, _: &str, _: &[u8]) -> Result<String, DomainError> {
        panic!("PanicObjectStorage called")
    }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        panic!("PanicObjectStorage called")
    }
    async fn get_stream(
        &self,
        _: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        panic!("PanicObjectStorage called")
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        panic!("PanicObjectStorage called")
    }
    async fn exists(&self, _: &str) -> Result<bool, DomainError> {
        panic!("PanicObjectStorage called")
    }
}

pub struct PanicAuthService;

#[async_trait]
impl AuthService for PanicAuthService {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!("PanicAuthService called")
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        panic!("PanicAuthService called")
    }
}

pub struct PanicPasswordHasher;

#[async_trait]
impl PasswordHasher for PanicPasswordHasher {
    async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> {
        panic!("PanicPasswordHasher called")
    }
    async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> {
        panic!("PanicPasswordHasher called")
    }
}

pub struct PanicUserRepository;

#[async_trait]
impl UserRepository for PanicUserRepository {
    async fn find_by_email(&self, _: &Email) -> Result<Option<crate::models::User>, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn save(&self, _: &crate::models::User) -> Result<(), DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<crate::models::User>, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn find_by_username(
        &self,
        _: &crate::value_objects::Username,
    ) -> Result<Option<crate::models::User>, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn list_with_stats(&self) -> Result<Vec<crate::models::UserSummary>, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn update_profile(
        &self,
        _: &UserId,
        _: &crate::models::UserProfile,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn mark_deleted(&self, _: &UserId, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn find_tombstoned_by_id(
        &self,
        _: &UserId,
    ) -> Result<Option<crate::models::User>, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn is_username_retired(
        &self,
        _: &crate::value_objects::Username,
    ) -> Result<bool, DomainError> {
        panic!("PanicUserRepository called")
    }
    async fn count_active_admins(&self) -> Result<u64, DomainError> {
        panic!("PanicUserRepository called")
    }
}

pub struct PanicEventPublisher;

#[async_trait]
impl EventPublisher for PanicEventPublisher {
    async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
        panic!("PanicEventPublisher called")
    }
}

pub struct PanicRssFeedRenderer;

impl crate::ports::RssFeedRenderer for PanicRssFeedRenderer {
    fn render_feed(
        &self,
        _: &[DiaryEntry],
        _: &crate::ports::FeedChannel<'_>,
    ) -> Result<String, DomainError> {
        panic!("PanicRssFeedRenderer called")
    }
}

pub struct PanicGoalCommand;

#[async_trait]
impl crate::ports::GoalCommand for PanicGoalCommand {
    async fn save(&self, _: &crate::models::Goal, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicGoalCommand called")
    }
    async fn update(&self, _: &crate::models::Goal, _: &[DomainEvent]) -> Result<(), DomainError> {
        panic!("PanicGoalCommand called")
    }
    async fn delete(
        &self,
        _: &crate::value_objects::GoalId,
        _: &crate::value_objects::UserId,
        _: &[DomainEvent],
    ) -> Result<(), DomainError> {
        panic!("PanicGoalCommand called")
    }
}

pub struct PanicGoalQuery;

#[async_trait]
impl crate::ports::GoalQuery for PanicGoalQuery {
    async fn find_by_user_and_year(
        &self,
        _: &crate::value_objects::UserId,
        _: u16,
    ) -> Result<Option<crate::models::Goal>, DomainError> {
        panic!("PanicGoalQuery called")
    }
    async fn list_for_user(
        &self,
        _: &crate::value_objects::UserId,
    ) -> Result<Vec<crate::models::Goal>, DomainError> {
        panic!("PanicGoalQuery called")
    }
}

pub struct PanicUserSettingsRepository;

#[async_trait]
impl crate::ports::UserSettingsRepository for PanicUserSettingsRepository {
    async fn get(
        &self,
        _: &crate::value_objects::UserId,
    ) -> Result<crate::models::UserSettings, DomainError> {
        panic!("PanicUserSettingsRepository called")
    }
    async fn save(&self, _: &crate::models::UserSettings) -> Result<(), DomainError> {
        panic!("PanicUserSettingsRepository called")
    }
}

pub struct PanicRemoteGoalRepository;

#[async_trait]
impl crate::ports::RemoteGoalRepository for PanicRemoteGoalRepository {
    async fn save(&self, _: crate::models::RemoteGoalEntry) -> Result<(), DomainError> {
        panic!("PanicRemoteGoalRepository called")
    }
    async fn update_by_ap_id(&self, _: &str, _: u32, _: u32) -> Result<(), DomainError> {
        panic!("PanicRemoteGoalRepository called")
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        panic!("PanicRemoteGoalRepository called")
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        panic!("PanicRemoteGoalRepository called")
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteGoalEntry>, DomainError> {
        panic!("PanicRemoteGoalRepository called")
    }
}
