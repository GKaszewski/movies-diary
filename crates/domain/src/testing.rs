#![cfg(any(test, feature = "test-helpers"))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, EntityType, ExportFormat, ExternalPersonId,
        FeedEntry, FieldMapping, FileFormat, ImportError, ImportProfile, ImportSession,
        IndexableDocument, Movie, MovieFilter, MovieProfile, MovieStats, MovieSummary, ParsedFile,
        Person, PersonCredits, PersonId, Review, ReviewHistory, SearchQuery, SearchResults, User,
        UserStats, UserSummary, UserTrends, WatchlistEntry, WatchlistWithMovie,
        collections::{PageParams, Paginated},
    },
    ports::{
        AuthService, DiaryExporter, DiaryRepository, DocumentParser, EventPublisher, FeedSortBy,
        FollowingFilter, GeneratedToken, ImageStorage, ImportProfileRepository,
        ImportSessionRepository, MetadataClient, MetadataSearchCriteria, MovieProfileRepository,
        MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
        ReviewRepository, SearchCommand, SearchPort, StatsRepository, UserProfileFieldsRepository,
        UserRepository, WatchlistRepository, WrapUpRepository,
    },
    value_objects::{
        Email, ExternalMetadataId, ImportProfileId, ImportSessionId, MovieId, MovieTitle,
        PasswordHash, PosterUrl, ReleaseYear, ReviewId, UserId, Username, WrapUpId,
    },
};

// ── InMemoryMovieRepository ───────────────────────────────────────────────────

pub struct InMemoryMovieRepository {
    pub store: Mutex<HashMap<Uuid, Movie>>,
}

impl InMemoryMovieRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl MovieRepository for InMemoryMovieRepository {
    async fn get_movie_by_external_id(
        &self,
        external_metadata_id: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|m| {
                m.external_metadata_id()
                    .map(|e| e.value() == external_metadata_id.value())
                    .unwrap_or(false)
            })
            .cloned())
    }

    async fn get_movie_by_id(&self, movie_id: &MovieId) -> Result<Option<Movie>, DomainError> {
        Ok(self.store.lock().unwrap().get(&movie_id.value()).cloned())
    }

    async fn get_movies_by_title_and_year(
        &self,
        title: &MovieTitle,
        year: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|m| m.title() == title && m.release_year() == year)
            .cloned()
            .collect())
    }

    async fn upsert_movie(&self, movie: &Movie) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(movie.id().value(), movie.clone());
        Ok(())
    }

    async fn delete_movie(&self, movie_id: &MovieId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&movie_id.value());
        Ok(())
    }

    async fn existing_external_ids(
        &self,
        ids: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        let store = self.store.lock().unwrap();
        let known: std::collections::HashSet<String> = store
            .values()
            .filter_map(|m| m.external_metadata_id().map(|e| e.value().to_string()))
            .collect();
        Ok(ids
            .iter()
            .map(|id| id.value().to_string())
            .filter(|v| known.contains(v))
            .collect())
    }

    async fn existing_title_year_pairs(
        &self,
        pairs: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        let store = self.store.lock().unwrap();
        let known: std::collections::HashSet<(String, u16)> = store
            .values()
            .map(|m| (m.title().value().to_string(), m.release_year().value()))
            .collect();
        Ok(pairs
            .iter()
            .map(|(t, y)| (t.value().to_string(), y.value()))
            .filter(|p| known.contains(p))
            .collect())
    }

    async fn list_movies(
        &self,
        _page: &crate::models::collections::PageParams,
        _filter: &MovieFilter,
    ) -> Result<Paginated<MovieSummary>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }
}

// ── InMemoryReviewRepository ──────────────────────────────────────────────────

pub struct InMemoryReviewRepository {
    store: Mutex<HashMap<Uuid, Review>>,
}

impl InMemoryReviewRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl ReviewRepository for InMemoryReviewRepository {
    async fn save_review(&self, review: &Review) -> Result<DomainEvent, DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(review.id().value(), review.clone());
        Ok(DomainEvent::ReviewLogged {
            review_id: review.id().clone(),
            movie_id: review.movie_id().clone(),
            user_id: review.user_id().clone(),
            rating: review.rating().clone(),
            watched_at: *review.watched_at(),
        })
    }

    async fn get_review_by_id(&self, review_id: &ReviewId) -> Result<Option<Review>, DomainError> {
        Ok(self.store.lock().unwrap().get(&review_id.value()).cloned())
    }

    async fn delete_review(&self, review_id: &ReviewId) -> Result<(), DomainError> {
        self.store.lock().unwrap().remove(&review_id.value());
        Ok(())
    }

    async fn get_all_reviews_for_user(&self, user_id: &UserId) -> Result<Vec<Review>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .filter(|r| r.user_id() == user_id)
            .cloned()
            .collect())
    }
}

// ── InMemoryUserRepository ────────────────────────────────────────────────────

pub struct InMemoryUserRepository {
    pub store: Mutex<HashMap<Uuid, User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|u| u.email().value() == email.value())
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .values()
            .find(|u| u.username().value() == username.value())
            .cloned())
    }

    async fn save(&self, user: &User) -> Result<(), DomainError> {
        self.store
            .lock()
            .unwrap()
            .insert(user.id().value(), user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, DomainError> {
        Ok(self.store.lock().unwrap().get(&id.value()).cloned())
    }

    async fn list_with_stats(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(vec![])
    }

    async fn update_profile(
        &self,
        _user_id: &UserId,
        _profile: &crate::models::UserProfile,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── InMemoryWatchlistRepository ───────────────────────────────────────────────

pub struct InMemoryWatchlistRepository {
    store: Mutex<HashMap<(Uuid, Uuid), WatchlistEntry>>,
}

impl InMemoryWatchlistRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(HashMap::new()),
        })
    }

    pub fn count(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

#[async_trait]
impl WatchlistRepository for InMemoryWatchlistRepository {
    async fn add(&self, entry: &WatchlistEntry) -> Result<(), DomainError> {
        let key = (entry.user_id.value(), entry.movie_id.value());
        self.store
            .lock()
            .unwrap()
            .entry(key)
            .or_insert_with(|| entry.clone());
        Ok(())
    }

    async fn remove(&self, user_id: &UserId, movie_id: &MovieId) -> Result<(), DomainError> {
        let key = (user_id.value(), movie_id.value());
        self.store
            .lock()
            .unwrap()
            .remove(&key)
            .ok_or_else(|| DomainError::NotFound("watchlist entry".into()))?;
        Ok(())
    }

    async fn remove_if_present(
        &self,
        user_id: &UserId,
        movie_id: &MovieId,
    ) -> Result<bool, DomainError> {
        let key = (user_id.value(), movie_id.value());
        Ok(self.store.lock().unwrap().remove(&key).is_some())
    }

    async fn get_for_user(
        &self,
        _user_id: &UserId,
        _page: &PageParams,
    ) -> Result<Paginated<WatchlistWithMovie>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn contains(&self, user_id: &UserId, movie_id: &MovieId) -> Result<bool, DomainError> {
        let key = (user_id.value(), movie_id.value());
        Ok(self.store.lock().unwrap().contains_key(&key))
    }
}

// ── NoopEventPublisher ────────────────────────────────────────────────────────

pub struct NoopEventPublisher {
    pub events: Mutex<Vec<DomainEvent>>,
}

impl NoopEventPublisher {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(vec![]),
        })
    }

    pub fn published(&self) -> Vec<DomainEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), DomainError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

// ── NoopImageStorage ──────────────────────────────────────────────────────────

pub struct NoopImageStorage;

#[async_trait]
impl ImageStorage for NoopImageStorage {
    async fn store(&self, key: &str, _image_bytes: &[u8]) -> Result<String, DomainError> {
        Ok(format!("noop://{key}"))
    }

    async fn get(&self, _key: &str) -> Result<Vec<u8>, DomainError> {
        Ok(vec![])
    }

    async fn get_stream(
        &self,
        _key: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn delete(&self, _key: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

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

// ── PanicDiaryRepository ──────────────────────────────────────────────────────

pub struct PanicDiaryRepository;

#[async_trait]
impl DiaryRepository for PanicDiaryRepository {
    async fn query_diary(
        &self,
        _filter: &DiaryFilter,
    ) -> Result<Paginated<DiaryEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn query_activity_feed(
        &self,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn query_activity_feed_filtered(
        &self,
        _page: &PageParams,
        _sort_by: &FeedSortBy,
        _search: Option<&str>,
        _following: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn get_review_history(&self, _movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn get_user_history(&self, _user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn get_movie_stats(&self, _movie_id: &MovieId) -> Result<MovieStats, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn get_movie_social_feed(
        &self,
        _movie_id: &MovieId,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!("PanicDiaryRepository called")
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        panic!("PanicDiaryRepository called")
    }
}

// ── PanicStatsRepository ──────────────────────────────────────────────────────

pub struct PanicStatsRepository;

#[async_trait]
impl StatsRepository for PanicStatsRepository {
    async fn get_user_stats(&self, _user_id: &UserId) -> Result<UserStats, DomainError> {
        panic!("PanicStatsRepository called")
    }

    async fn get_user_trends(&self, _user_id: &UserId) -> Result<UserTrends, DomainError> {
        panic!("PanicStatsRepository called")
    }
}

// ── PanicImportSessionRepository ──────────────────────────────────────────────

pub struct PanicImportSessionRepository;

#[async_trait]
impl ImportSessionRepository for PanicImportSessionRepository {
    async fn create(&self, _session: &ImportSession) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }

    async fn get(
        &self,
        _id: &ImportSessionId,
        _user_id: &UserId,
    ) -> Result<Option<ImportSession>, DomainError> {
        panic!("PanicImportSessionRepository called")
    }

    async fn update(&self, _session: &ImportSession) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }

    async fn delete(&self, _id: &ImportSessionId) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }

    async fn delete_expired(&self) -> Result<u64, DomainError> {
        panic!("PanicImportSessionRepository called")
    }

    async fn delete_expired_for_user(&self, _user_id: &UserId) -> Result<(), DomainError> {
        panic!("PanicImportSessionRepository called")
    }
}

// ── PanicImportProfileRepository ──────────────────────────────────────────────

pub struct PanicImportProfileRepository;

#[async_trait]
impl ImportProfileRepository for PanicImportProfileRepository {
    async fn save(&self, _profile: &ImportProfile) -> Result<(), DomainError> {
        panic!("PanicImportProfileRepository called")
    }

    async fn list_for_user(&self, _user_id: &UserId) -> Result<Vec<ImportProfile>, DomainError> {
        panic!("PanicImportProfileRepository called")
    }

    async fn get(
        &self,
        _id: &ImportProfileId,
        _user_id: &UserId,
    ) -> Result<Option<ImportProfile>, DomainError> {
        panic!("PanicImportProfileRepository called")
    }

    async fn delete(&self, _id: &ImportProfileId) -> Result<(), DomainError> {
        panic!("PanicImportProfileRepository called")
    }
}

// ── PanicMovieProfileRepository ───────────────────────────────────────────────

pub struct PanicMovieProfileRepository;

#[async_trait]
impl MovieProfileRepository for PanicMovieProfileRepository {
    async fn upsert(&self, _profile: &MovieProfile) -> Result<(), DomainError> {
        panic!("PanicMovieProfileRepository called")
    }

    async fn get_by_movie_id(&self, _id: &MovieId) -> Result<Option<MovieProfile>, DomainError> {
        panic!("PanicMovieProfileRepository called")
    }

    async fn list_stale(&self) -> Result<Vec<(MovieId, String)>, DomainError> {
        panic!("PanicMovieProfileRepository called")
    }
}

// ── PanicPersonCommand ────────────────────────────────────────────────────────

pub struct PanicPersonCommand;

#[async_trait]
impl PersonCommand for PanicPersonCommand {
    async fn upsert_batch(&self, _persons: &[Person]) -> Result<(), DomainError> {
        panic!("PanicPersonCommand called")
    }
}

// ── PanicPersonQuery ──────────────────────────────────────────────────────────

pub struct PanicPersonQuery;

#[async_trait]
impl PersonQuery for PanicPersonQuery {
    async fn get_by_id(&self, _id: &PersonId) -> Result<Option<Person>, DomainError> {
        panic!("PanicPersonQuery called")
    }

    async fn get_by_external_id(
        &self,
        _id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        panic!("PanicPersonQuery called")
    }

    async fn get_credits(&self, _id: &PersonId) -> Result<PersonCredits, DomainError> {
        panic!("PanicPersonQuery called")
    }

    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        panic!("PanicPersonQuery called")
    }
}

// ── PanicSearchPort ───────────────────────────────────────────────────────────

pub struct PanicSearchPort;

#[async_trait]
impl SearchPort for PanicSearchPort {
    async fn search(&self, _query: &SearchQuery) -> Result<SearchResults, DomainError> {
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

// ── PanicSearchCommand ────────────────────────────────────────────────────────

pub struct PanicSearchCommand;

#[async_trait]
impl SearchCommand for PanicSearchCommand {
    async fn index(&self, _doc: IndexableDocument) -> Result<(), DomainError> {
        panic!("PanicSearchCommand called")
    }

    async fn remove(&self, _entity_type: EntityType, _id: &str) -> Result<(), DomainError> {
        panic!("PanicSearchCommand called")
    }
}

// ── PanicPosterFetcher ────────────────────────────────────────────────────────

pub struct PanicPosterFetcher;

#[async_trait]
impl PosterFetcherClient for PanicPosterFetcher {
    async fn fetch_poster_bytes(&self, _poster_url: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        panic!("PanicPosterFetcher called")
    }
}

// ── PanicDiaryExporter ────────────────────────────────────────────────────────

pub struct PanicDiaryExporter;

#[async_trait]
impl DiaryExporter for PanicDiaryExporter {
    async fn serialize_entries(
        &self,
        _entries: &[crate::models::DiaryEntry],
        _format: ExportFormat,
    ) -> Result<Vec<u8>, DomainError> {
        panic!("PanicDiaryExporter called")
    }
}

// ── PanicDocumentParser ───────────────────────────────────────────────────────

pub struct PanicDocumentParser;

impl DocumentParser for PanicDocumentParser {
    fn parse(&self, _bytes: &[u8], _format: FileFormat) -> Result<ParsedFile, ImportError> {
        panic!("PanicDocumentParser called")
    }

    fn apply_mapping(&self, _file: &ParsedFile, _mappings: &[FieldMapping]) -> Vec<AnnotatedRow> {
        panic!("PanicDocumentParser called")
    }
}

// ── PanicProfileFieldsRepo ────────────────────────────────────────────────────

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

pub struct NoopRemoteWatchlistRepository;

#[async_trait]
impl crate::ports::RemoteWatchlistRepository for NoopRemoteWatchlistRepository {
    async fn save(&self, _: crate::models::RemoteWatchlistEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_derived_uuid(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
}

pub struct PanicProfileFieldsRepo;

#[async_trait]
impl UserProfileFieldsRepository for PanicProfileFieldsRepo {
    async fn get_fields(
        &self,
        _user_id: &UserId,
    ) -> Result<Vec<crate::models::ProfileField>, DomainError> {
        panic!("PanicProfileFieldsRepo called")
    }

    async fn set_fields(
        &self,
        _user_id: &UserId,
        _fields: Vec<crate::models::ProfileField>,
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

pub struct NoopSocialQueryPort;

#[async_trait]
impl crate::ports::SocialQueryPort for NoopSocialQueryPort {
    async fn get_accepted_following_urls(&self, _: uuid::Uuid) -> Result<Vec<String>, DomainError> {
        Ok(vec![])
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<crate::ports::RemoteActorInfo>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_accepted_followers(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_pending_followers(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<crate::ports::PendingFollowerInfo>, DomainError> {
        Ok(vec![])
    }
}

// ── PanicWatchEventRepository ────────────────────────────────────────────────

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

// ── PanicWebhookTokenRepository ──────────────────────────────────────────────

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

// ── PanicWrapUpStatsQuery ───────────────────────────────────────────────────

pub struct PanicWrapUpStatsQuery;

#[async_trait]
impl crate::ports::WrapUpStatsQuery for PanicWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        _scope: &crate::models::wrapup::WrapUpScope,
        _range: &crate::models::wrapup::DateRange,
    ) -> Result<Vec<crate::ports::WrapUpMovieRow>, DomainError> {
        unimplemented!("WrapUpStatsQuery not wired")
    }
}

// ── InMemoryWrapUpStatsQuery ────────────────────────────────────────────────

pub struct InMemoryWrapUpStatsQuery {
    pub rows: Mutex<Vec<crate::ports::WrapUpMovieRow>>,
}

impl InMemoryWrapUpStatsQuery {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rows: Mutex::new(Vec::new()),
        })
    }

    pub fn with_rows(rows: Vec<crate::ports::WrapUpMovieRow>) -> Arc<Self> {
        Arc::new(Self {
            rows: Mutex::new(rows),
        })
    }
}

#[async_trait]
impl crate::ports::WrapUpStatsQuery for InMemoryWrapUpStatsQuery {
    async fn get_reviews_with_profiles(
        &self,
        scope: &crate::models::wrapup::WrapUpScope,
        range: &crate::models::wrapup::DateRange,
    ) -> Result<Vec<crate::ports::WrapUpMovieRow>, DomainError> {
        let rows = self.rows.lock().unwrap();
        let filtered: Vec<_> = rows
            .iter()
            .filter(|r| {
                let date = r.watched_at.date();
                date >= range.start && date < range.end
            })
            .filter(|r| match scope {
                crate::models::wrapup::WrapUpScope::User(uid) => r.user_id == *uid,
                crate::models::wrapup::WrapUpScope::Global => true,
            })
            .cloned()
            .collect();
        Ok(filtered)
    }
}

// ── InMemoryWrapUpRepository ────────────────────────────────────────────────

pub struct InMemoryWrapUpRepository {
    pub store: Mutex<Vec<crate::models::wrapup::WrapUpRecord>>,
}

impl InMemoryWrapUpRepository {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl WrapUpRepository for InMemoryWrapUpRepository {
    async fn create(
        &self,
        record: &crate::models::wrapup::WrapUpRecord,
    ) -> Result<(), DomainError> {
        self.store.lock().unwrap().push(record.clone());
        Ok(())
    }

    async fn update_status(
        &self,
        id: &WrapUpId,
        status: &crate::models::wrapup::WrapUpStatus,
        error: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        if let Some(rec) = store.iter_mut().find(|r| r.id == *id) {
            rec.status = status.clone();
            rec.error_message = error.map(|s| s.to_string());
            Ok(())
        } else {
            Err(DomainError::NotFound("wrapup record".into()))
        }
    }

    async fn set_complete(&self, id: &WrapUpId, report_json: &str) -> Result<(), DomainError> {
        let mut store = self.store.lock().unwrap();
        if let Some(rec) = store.iter_mut().find(|r| r.id == *id) {
            rec.status = crate::models::wrapup::WrapUpStatus::Ready;
            rec.report_json = Some(report_json.to_string());
            rec.completed_at = Some(chrono::Utc::now().naive_utc());
            Ok(())
        } else {
            Err(DomainError::NotFound("wrapup record".into()))
        }
    }

    async fn get_by_id(
        &self,
        id: &WrapUpId,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store.iter().find(|r| r.id == *id).cloned())
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|r| r.user_id == Some(user_id))
            .cloned()
            .collect())
    }

    async fn list_global(&self) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .filter(|r| r.user_id.is_none())
            .cloned()
            .collect())
    }

    async fn find_existing(
        &self,
        user_id: Option<Uuid>,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        let store = self.store.lock().unwrap();
        Ok(store
            .iter()
            .find(|r| r.user_id == user_id && r.start_date == start && r.end_date == end)
            .cloned())
    }
}

// ── PanicWrapUpRepository ──────────────────────────────────────────────────

pub struct PanicWrapUpRepository;

#[async_trait]
impl WrapUpRepository for PanicWrapUpRepository {
    async fn create(&self, _: &crate::models::wrapup::WrapUpRecord) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn update_status(
        &self,
        _: &WrapUpId,
        _: &crate::models::wrapup::WrapUpStatus,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn set_complete(&self, _: &WrapUpId, _: &str) -> Result<(), DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn get_by_id(
        &self,
        _: &WrapUpId,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn list_for_user(
        &self,
        _: Uuid,
    ) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn list_global(&self) -> Result<Vec<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
    async fn find_existing(
        &self,
        _: Option<Uuid>,
        _: chrono::NaiveDate,
        _: chrono::NaiveDate,
    ) -> Result<Option<crate::models::wrapup::WrapUpRecord>, DomainError> {
        panic!("PanicWrapUpRepository called")
    }
}
