use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    errors::DomainError,
    models::{
        AnnotatedRow, DiaryEntry, DiaryFilter, ExternalPersonId, FeedEntry, FeedSortBy,
        FieldMapping, FileFormat, FollowingFilter, GeneratedToken, ImportError, ImportRow,
        MetadataSearchCriteria, Movie, MovieProfile, MovieStats, ParsedFile, Person, PersonCredits,
        PersonId, Review, ReviewHistory, RowResult, SearchQuery, SearchResults, UserStats,
        UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::{
        AuthService, DiaryRepository, DocumentParser, MetadataClient, MovieEnrichmentClient,
        PasswordHasher, PersonQuery, PosterFetcherClient, SearchCommand, SearchPort,
        StatsRepository,
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
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn query_activity_feed(
        &self,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn query_activity_feed_filtered(
        &self,
        _page: &PageParams,
        _sort_by: &FeedSortBy,
        _search: Option<&str>,
        _following: Option<&FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn get_review_history(&self, movie_id: &MovieId) -> Result<ReviewHistory, DomainError> {
        let histories = self.histories.lock().unwrap();
        let (movie, reviews) = histories
            .get(&movie_id.value())
            .ok_or_else(|| DomainError::NotFound(format!("movie {}", movie_id.value())))?;
        Ok(ReviewHistory::new(movie.clone(), reviews.clone()))
    }

    async fn get_user_history(&self, _user_id: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        Ok(vec![])
    }

    fn stream_user_history(
        &self,
        _user_id: UserId,
    ) -> futures::stream::BoxStream<'static, Result<DiaryEntry, DomainError>> {
        Box::pin(futures::stream::empty())
    }

    async fn get_movie_stats(&self, _movie_id: &MovieId) -> Result<MovieStats, DomainError> {
        Ok(MovieStats {
            total_count: 0,
            avg_rating: None,
            federated_count: 0,
            rating_histogram: [0; 5],
        })
    }

    async fn get_movie_social_feed(
        &self,
        _movie_id: &MovieId,
        _page: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        Ok(Paginated {
            items: vec![],
            total_count: 0,
            limit: 10,
            offset: 0,
        })
    }

    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
}

// ── FakeStatsRepository ─────────────────────────────────────────────────────

pub struct FakeStatsRepository;

#[async_trait]
impl StatsRepository for FakeStatsRepository {
    async fn get_user_stats(&self, _: &UserId) -> Result<UserStats, DomainError> {
        Ok(UserStats {
            total_movies: 0,
            avg_rating: None,
            favorite_director: None,
            most_active_month: None,
        })
    }

    async fn get_user_trends(&self, _: &UserId) -> Result<UserTrends, DomainError> {
        Ok(UserTrends {
            monthly_ratings: vec![],
            top_directors: vec![],
            max_director_count: 0,
        })
    }
}

// ── FakePersonQuery ─────────────────────────────────────────────────────────

pub struct FakePersonQuery;

#[async_trait]
impl PersonQuery for FakePersonQuery {
    async fn get_by_id(&self, _: &PersonId) -> Result<Option<Person>, DomainError> {
        Ok(None)
    }

    async fn get_by_external_id(
        &self,
        _: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        Ok(None)
    }

    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError> {
        let dummy = Person::basic(
            id.clone(),
            ExternalPersonId::new("tmdb:0"),
            "Unknown".into(),
            None,
            None,
        );
        Ok(PersonCredits {
            person: dummy,
            cast: vec![],
            crew: vec![],
        })
    }

    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        Ok(vec![])
    }

    async fn list_page(&self, _: u32, _: u32) -> Result<Vec<Person>, DomainError> {
        Ok(vec![])
    }
}

// ── FakeSearchPort ──────────────────────────────────────────────────────────

pub struct FakeSearchPort;

#[async_trait]
impl SearchPort for FakeSearchPort {
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

// ── FakeSearchCommand ───────────────────────────────────────────────────────

pub struct FakeSearchCommand;

#[async_trait]
impl SearchCommand for FakeSearchCommand {
    async fn index(&self, _: crate::models::IndexableDocument) -> Result<(), DomainError> {
        Ok(())
    }

    async fn remove(&self, _: crate::models::EntityType, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ── FakeDocumentParser ──────────────────────────────────────────────────────

pub struct FakeDocumentParser;

impl DocumentParser for FakeDocumentParser {
    fn parse(&self, _: &[u8], _: FileFormat) -> Result<ParsedFile, ImportError> {
        Ok(ParsedFile {
            columns: vec!["title".into()],
            rows: vec![vec!["Test Movie".into()]],
        })
    }

    fn apply_mapping(&self, _: &ParsedFile, _: &[FieldMapping]) -> Vec<AnnotatedRow> {
        vec![AnnotatedRow {
            result: RowResult::Valid(ImportRow {
                title: Some("Test Movie".into()),
                ..ImportRow::default()
            }),
            is_duplicate: false,
        }]
    }
}

// ── FakePosterFetcher ───────────────────────────────────────────────────────

pub struct FakePosterFetcher;

#[async_trait]
impl PosterFetcherClient for FakePosterFetcher {
    async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        Ok(vec![1, 2, 3])
    }
}

// ── FakeMovieEnrichmentClient ───────────────────────────────────────────────

pub struct FakeMovieEnrichmentClient;

#[async_trait]
impl MovieEnrichmentClient for FakeMovieEnrichmentClient {
    async fn fetch_profile(
        &self,
        movie_id: MovieId,
        _external_metadata_id: &str,
    ) -> Result<MovieProfile, DomainError> {
        Ok(MovieProfile {
            movie_id,
            tmdb_id: 0,
            imdb_id: None,
            overview: None,
            tagline: None,
            runtime_minutes: None,
            budget_usd: None,
            revenue_usd: None,
            vote_average: None,
            vote_count: None,
            original_language: None,
            collection_name: None,
            genres: vec![],
            keywords: vec![],
            cast: vec![],
            crew: vec![],
            enriched_at: Utc::now(),
        })
    }
}
