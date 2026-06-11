use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, EntityType, ExportFormat, ExternalPersonId,
        FeedEntry, FieldMapping, FileFormat, ImportError, ImportProfile, ImportSession,
        IndexableDocument, MovieProfile, MovieStats, ParsedFile, Person, PersonCredits,
        PersonEnrichmentData, PersonId,
        ReviewHistory, SearchQuery, SearchResults, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::{
        DiaryExporter, DiaryRepository, DocumentParser, FeedSortBy, FollowingFilter,
        ImportProfileRepository, ImportSessionRepository, MovieProfileRepository, PersonCommand,
        PersonQuery, PosterFetcherClient, SearchCommand, SearchPort, StatsRepository,
        UserProfileFieldsRepository,
    },
    value_objects::{ImportProfileId, ImportSessionId, MovieId, PosterUrl, UserId},
};

// ── PanicDiaryRepository ──────────────────────────────────────────────────────

pub struct PanicDiaryRepository;

#[async_trait]
impl DiaryRepository for PanicDiaryRepository {
    async fn query_diary(&self, _: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn query_activity_feed(
        &self,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn query_activity_feed_filtered(
        &self,
        _: &PageParams,
        _: &FeedSortBy,
        _: Option<&str>,
        _: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn get_review_history(&self, _: &MovieId) -> Result<ReviewHistory, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn get_user_history(&self, _: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn get_movie_stats(&self, _: &MovieId) -> Result<MovieStats, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn get_movie_social_feed(
        &self,
        _: &MovieId,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }
    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        panic!("PanicDiaryRepository called")
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

#[async_trait]
impl DiaryExporter for PanicDiaryExporter {
    async fn serialize_entries(
        &self,
        _: &[DiaryEntry],
        _: ExportFormat,
    ) -> Result<Vec<u8>, DomainError> {
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
    ) -> Result<(), DomainError> {
        panic!("PanicProfileFieldsRepo called")
    }
}

pub struct PanicSocialQueryPort;

#[async_trait]
impl crate::ports::SocialQueryPort for PanicSocialQueryPort {
    async fn get_accepted_following_urls(&self, _: uuid::Uuid) -> Result<Vec<String>, DomainError> {
        panic!("PanicSocialQueryPort called")
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<crate::ports::RemoteActorInfo>, DomainError> {
        panic!("PanicSocialQueryPort called")
    }
    async fn count_following(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        panic!("PanicSocialQueryPort called")
    }
    async fn count_accepted_followers(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        panic!("PanicSocialQueryPort called")
    }
    async fn get_pending_followers(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::ports::PendingFollowerInfo>, DomainError> {
        panic!("PanicSocialQueryPort called")
    }
}

pub struct PanicWatchEventRepository;

#[async_trait]
impl crate::ports::WatchEventRepository for PanicWatchEventRepository {
    async fn save(&self, _: &crate::models::WatchEvent) -> Result<(), DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn update_status(
        &self,
        _: &crate::value_objects::WatchEventId,
        _: crate::models::WatchEventStatus,
    ) -> Result<(), DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn list_pending(
        &self,
        _: &UserId,
    ) -> Result<Vec<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn get_by_id(
        &self,
        _: &crate::value_objects::WatchEventId,
    ) -> Result<Option<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn get_by_ids(
        &self,
        _: &[crate::value_objects::WatchEventId],
    ) -> Result<Vec<crate::models::WatchEvent>, DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn update_status_batch(
        &self,
        _: &[crate::value_objects::WatchEventId],
        _: crate::models::WatchEventStatus,
    ) -> Result<u64, DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn find_duplicate(
        &self,
        _: &UserId,
        _: &str,
        _: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        panic!("PanicWatchEventRepository called")
    }
    async fn delete_non_pending_older_than(
        &self,
        _: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        panic!("PanicWatchEventRepository called")
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
