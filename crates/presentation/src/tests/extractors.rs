use super::*;
use application::{
    config::AppConfig,
    context::{AppContext, Repositories, Services},
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use domain::{
    errors::DomainError,
    events::DomainEvent,
    models::{
        DiaryEntry, DiaryFilter, EntityType, FeedEntry, IndexableDocument, Movie, Person,
        PersonCredits, PersonId, Review, ReviewHistory, SearchQuery, SearchResults, UserStats,
        UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::{
        AuthService, DiaryRepository, EventPublisher, GeneratedToken, ImageStorage, MetadataClient,
        MovieRepository, PasswordHasher, PersonCommand, PersonQuery, PosterFetcherClient,
        ReviewRepository, SearchCommand, SearchPort, StatsRepository, UserRepository,
        WatchlistRepository,
    },
    value_objects::{
        Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterUrl, ReleaseYear,
        ReviewId, UserId,
    },
};
use std::sync::Arc;
use tower::ServiceExt;

// --- Panic stubs (defined once) ---

pub struct Panic;

#[async_trait::async_trait]
impl MovieRepository for Panic {
    async fn get_movie_by_external_id(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<Movie>, DomainError> {
        panic!()
    }
    async fn get_movie_by_id(&self, _: &MovieId) -> Result<Option<Movie>, DomainError> {
        panic!()
    }
    async fn get_movies_by_title_and_year(
        &self,
        _: &MovieTitle,
        _: &ReleaseYear,
    ) -> Result<Vec<Movie>, DomainError> {
        panic!()
    }
    async fn upsert_movie(&self, _: &Movie) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete_movie(&self, _: &MovieId) -> Result<(), DomainError> {
        panic!()
    }
    async fn existing_external_ids(
        &self,
        _: &[ExternalMetadataId],
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        panic!()
    }
    async fn existing_title_year_pairs(
        &self,
        _: &[(MovieTitle, ReleaseYear)],
    ) -> Result<std::collections::HashSet<(String, u16)>, DomainError> {
        panic!()
    }
    async fn list_movies(
        &self,
        _: &domain::models::collections::PageParams,
        _: &domain::models::MovieFilter,
    ) -> Result<domain::models::collections::Paginated<domain::models::MovieSummary>, DomainError>
    {
        panic!()
    }
}
#[async_trait::async_trait]
impl ReviewRepository for Panic {
    async fn save_review(&self, _: &Review) -> Result<DomainEvent, DomainError> {
        panic!()
    }
    async fn get_review_by_id(&self, _: &ReviewId) -> Result<Option<Review>, DomainError> {
        panic!()
    }
    async fn delete_review(&self, _: &ReviewId) -> Result<(), DomainError> {
        panic!()
    }
    async fn get_all_reviews_for_user(&self, _: &UserId) -> Result<Vec<Review>, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl DiaryRepository for Panic {
    async fn query_diary(&self, _: &DiaryFilter) -> Result<Paginated<DiaryEntry>, DomainError> {
        panic!()
    }
    async fn query_activity_feed(
        &self,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!()
    }
    async fn query_activity_feed_filtered(
        &self,
        _: &PageParams,
        _: &domain::ports::FeedSortBy,
        _: Option<&str>,
        _: Option<&domain::ports::FollowingFilter>,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!()
    }
    async fn get_review_history(&self, _: &MovieId) -> Result<ReviewHistory, DomainError> {
        panic!()
    }
    async fn get_user_history(&self, _: &UserId) -> Result<Vec<DiaryEntry>, DomainError> {
        panic!()
    }
    async fn get_movie_stats(
        &self,
        _: &MovieId,
    ) -> Result<domain::models::MovieStats, DomainError> {
        panic!()
    }
    async fn get_movie_social_feed(
        &self,
        _: &MovieId,
        _: &PageParams,
    ) -> Result<Paginated<FeedEntry>, DomainError> {
        panic!()
    }
    async fn count_local_posts(&self) -> Result<u64, DomainError> {
        panic!()
    }
}
#[cfg(feature = "federation")]
#[async_trait::async_trait]
impl domain::ports::SocialQueryPort for Panic {
    async fn get_accepted_following_urls(&self, _: uuid::Uuid) -> Result<Vec<String>, DomainError> {
        panic!()
    }
    async fn list_all_followed_remote_actors(
        &self,
    ) -> Result<Vec<domain::ports::RemoteActorInfo>, DomainError> {
        panic!()
    }
    async fn count_following(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        panic!()
    }
    async fn count_accepted_followers(&self, _: uuid::Uuid) -> Result<usize, DomainError> {
        panic!()
    }
    async fn get_pending_followers(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<domain::ports::PendingFollowerInfo>, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl StatsRepository for Panic {
    async fn get_user_stats(&self, _: &UserId) -> Result<UserStats, DomainError> {
        panic!()
    }
    async fn get_user_trends(&self, _: &UserId) -> Result<UserTrends, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl MetadataClient for Panic {
    async fn fetch_movie_metadata(
        &self,
        _: &domain::ports::MetadataSearchCriteria,
    ) -> Result<Movie, DomainError> {
        panic!()
    }
    async fn get_poster_url(
        &self,
        _: &ExternalMetadataId,
    ) -> Result<Option<PosterUrl>, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl PosterFetcherClient for Panic {
    async fn fetch_poster_bytes(&self, _: &PosterUrl) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl ImageStorage for Panic {
    async fn store(&self, _: &str, _: &[u8]) -> Result<String, DomainError> {
        panic!()
    }
    async fn get(&self, _: &str) -> Result<Vec<u8>, DomainError> {
        panic!()
    }
    async fn get_stream(
        &self,
        _: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<bytes::Bytes, DomainError>>, DomainError>
    {
        panic!()
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl AuthService for Panic {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!()
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl PasswordHasher for Panic {
    async fn hash(&self, _: &str) -> Result<PasswordHash, DomainError> {
        panic!()
    }
    async fn verify(&self, _: &str, _: &PasswordHash) -> Result<bool, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl UserRepository for Panic {
    async fn find_by_email(&self, _: &Email) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn save(&self, _: &domain::models::User) -> Result<(), DomainError> {
        panic!()
    }
    async fn find_by_id(&self, _: &UserId) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn find_by_username(
        &self,
        _: &domain::value_objects::Username,
    ) -> Result<Option<domain::models::User>, DomainError> {
        panic!()
    }
    async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, DomainError> {
        panic!()
    }
    async fn update_profile(
        &self,
        _: &UserId,
        _: &domain::models::UserProfile,
    ) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl domain::ports::UserProfileFieldsRepository for Panic {
    async fn get_fields(
        &self,
        _: &UserId,
    ) -> Result<Vec<domain::models::ProfileField>, DomainError> {
        panic!()
    }
    async fn set_fields(
        &self,
        _: &UserId,
        _: Vec<domain::models::ProfileField>,
    ) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl EventPublisher for Panic {
    async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl domain::ports::ImportSessionRepository for Panic {
    async fn create(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> {
        panic!()
    }
    async fn get(
        &self,
        _: &domain::value_objects::ImportSessionId,
        _: &UserId,
    ) -> Result<Option<domain::models::ImportSession>, DomainError> {
        panic!()
    }
    async fn update(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete(&self, _: &domain::value_objects::ImportSessionId) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete_expired(&self) -> Result<u64, DomainError> {
        panic!()
    }
    async fn delete_expired_for_user(&self, _: &UserId) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl domain::ports::ImportProfileRepository for Panic {
    async fn save(&self, _: &domain::models::ImportProfile) -> Result<(), DomainError> {
        panic!()
    }
    async fn list_for_user(
        &self,
        _: &UserId,
    ) -> Result<Vec<domain::models::ImportProfile>, DomainError> {
        panic!()
    }
    async fn get(
        &self,
        _: &domain::value_objects::ImportProfileId,
        _: &UserId,
    ) -> Result<Option<domain::models::ImportProfile>, DomainError> {
        panic!()
    }
    async fn delete(&self, _: &domain::value_objects::ImportProfileId) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl WatchlistRepository for Panic {
    async fn add(&self, _: &domain::models::WatchlistEntry) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove_if_present(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_for_user(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::models::collections::PageParams,
    ) -> Result<
        domain::models::collections::Paginated<domain::models::WatchlistWithMovie>,
        DomainError,
    > {
        panic!()
    }
    async fn contains(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::MovieId,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
}
#[async_trait::async_trait]
impl domain::ports::MovieProfileRepository for Panic {
    async fn upsert(&self, _: &domain::models::MovieProfile) -> Result<(), DomainError> {
        panic!()
    }
    async fn get_by_movie_id(
        &self,
        _: &domain::value_objects::MovieId,
    ) -> Result<Option<domain::models::MovieProfile>, DomainError> {
        Ok(None)
    }
    async fn list_stale(
        &self,
    ) -> Result<Vec<(domain::value_objects::MovieId, String)>, DomainError> {
        Ok(vec![])
    }
}
#[async_trait::async_trait]
impl domain::ports::DiaryExporter for Panic {
    async fn serialize_entries(
        &self,
        _: &[domain::models::DiaryEntry],
        _: domain::models::ExportFormat,
    ) -> Result<Vec<u8>, domain::errors::DomainError> {
        panic!()
    }
}

impl domain::ports::DocumentParser for Panic {
    fn parse(
        &self,
        _: &[u8],
        _: domain::models::FileFormat,
    ) -> Result<domain::models::ParsedFile, domain::models::ImportError> {
        panic!()
    }
    fn apply_mapping(
        &self,
        _: &domain::models::ParsedFile,
        _: &[domain::models::FieldMapping],
    ) -> Vec<domain::models::AnnotatedRow> {
        panic!()
    }
}

impl crate::ports::RssFeedRenderer for Panic {
    fn render_feed(&self, _: &[DiaryEntry], _: &str) -> Result<String, String> {
        panic!()
    }
}

struct RejectingAuth;
#[async_trait::async_trait]
impl AuthService for RejectingAuth {
    async fn generate_token(&self, _: &UserId) -> Result<GeneratedToken, DomainError> {
        panic!()
    }
    async fn validate_token(&self, _: &str) -> Result<UserId, DomainError> {
        Err(DomainError::Unauthorized("bad token".into()))
    }
}

#[async_trait::async_trait]
impl PersonCommand for Panic {
    async fn upsert_batch(&self, _: &[Person]) -> Result<(), DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl PersonQuery for Panic {
    async fn get_by_id(&self, _: &PersonId) -> Result<Option<Person>, DomainError> {
        panic!()
    }
    async fn get_by_external_id(
        &self,
        _: &domain::models::ExternalPersonId,
    ) -> Result<Option<Person>, DomainError> {
        panic!()
    }
    async fn get_credits(&self, _: &PersonId) -> Result<PersonCredits, DomainError> {
        panic!()
    }
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl SearchPort for Panic {
    async fn search(&self, _: &SearchQuery) -> Result<SearchResults, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl SearchCommand for Panic {
    async fn index(&self, _: IndexableDocument) -> Result<(), DomainError> {
        panic!()
    }
    async fn remove(&self, _: EntityType, _: &str) -> Result<(), DomainError> {
        panic!()
    }
}
#[cfg(feature = "federation")]
#[async_trait::async_trait]
impl domain::ports::RemoteWatchlistRepository for Panic {
    async fn save(&self, _: domain::models::RemoteWatchlistEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_by_ap_id(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_actor_url(
        &self,
        _: &str,
    ) -> Result<Vec<domain::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn remove_all_by_actor(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_derived_uuid(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<domain::models::RemoteWatchlistEntry>, DomainError> {
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl domain::ports::WatchEventRepository for Panic {
    async fn save(&self, _: &domain::models::WatchEvent) -> Result<(), DomainError> {
        panic!()
    }
    async fn update_status(
        &self,
        _: &domain::value_objects::WatchEventId,
        _: domain::models::WatchEventStatus,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn list_pending(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::models::WatchEvent>, DomainError> {
        panic!()
    }
    async fn get_by_id(
        &self,
        _: &domain::value_objects::WatchEventId,
    ) -> Result<Option<domain::models::WatchEvent>, DomainError> {
        panic!()
    }
    async fn get_by_ids(
        &self,
        _: &[domain::value_objects::WatchEventId],
    ) -> Result<Vec<domain::models::WatchEvent>, DomainError> {
        panic!()
    }
    async fn update_status_batch(
        &self,
        _: &[domain::value_objects::WatchEventId],
        _: domain::models::WatchEventStatus,
    ) -> Result<u64, DomainError> {
        panic!()
    }
    async fn find_duplicate(
        &self,
        _: &domain::value_objects::UserId,
        _: &str,
        _: chrono::NaiveDateTime,
    ) -> Result<bool, DomainError> {
        panic!()
    }
    async fn delete_non_pending_older_than(
        &self,
        _: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        panic!()
    }
}
#[async_trait::async_trait]
impl domain::ports::WebhookTokenRepository for Panic {
    async fn save(&self, _: &domain::models::WebhookToken) -> Result<(), DomainError> {
        panic!()
    }
    async fn find_by_token_hash(
        &self,
        _: &str,
    ) -> Result<Option<domain::models::WebhookToken>, DomainError> {
        panic!()
    }
    async fn list_by_user(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::models::WebhookToken>, DomainError> {
        panic!()
    }
    async fn delete(
        &self,
        _: &domain::value_objects::WebhookTokenId,
        _: &domain::value_objects::UserId,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn touch_last_used(
        &self,
        _: &domain::value_objects::WebhookTokenId,
    ) -> Result<(), DomainError> {
        panic!()
    }
}

#[async_trait::async_trait]
impl domain::ports::WrapUpStatsQuery for Panic {
    async fn get_reviews_with_profiles(
        &self,
        _: &domain::models::wrapup::WrapUpScope,
        _: &domain::models::wrapup::DateRange,
    ) -> Result<Vec<domain::ports::WrapUpMovieRow>, DomainError> {
        panic!()
    }
}

#[async_trait::async_trait]
impl domain::ports::WrapUpRepository for Panic {
    async fn create(&self, _: &domain::models::wrapup::WrapUpRecord) -> Result<(), DomainError> {
        panic!()
    }
    async fn update_status(
        &self,
        _: &domain::value_objects::WrapUpId,
        _: &domain::models::wrapup::WrapUpStatus,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn set_complete(
        &self,
        _: &domain::value_objects::WrapUpId,
        _: &domain::models::wrapup::WrapUpReport,
    ) -> Result<(), DomainError> {
        panic!()
    }
    async fn get_by_id(
        &self,
        _: &domain::value_objects::WrapUpId,
    ) -> Result<Option<domain::models::wrapup::WrapUpRecord>, DomainError> {
        panic!()
    }
    async fn list_for_user(
        &self,
        _: uuid::Uuid,
    ) -> Result<Vec<domain::models::wrapup::WrapUpRecord>, DomainError> {
        panic!()
    }
    async fn list_global(&self) -> Result<Vec<domain::models::wrapup::WrapUpRecord>, DomainError> {
        panic!()
    }
    async fn find_existing(
        &self,
        _: Option<uuid::Uuid>,
        _: chrono::NaiveDate,
        _: chrono::NaiveDate,
    ) -> Result<Option<domain::models::wrapup::WrapUpRecord>, DomainError> {
        panic!()
    }
    async fn delete(&self, _: &domain::value_objects::WrapUpId) -> Result<(), DomainError> {
        panic!()
    }
    async fn delete_failed_older_than(&self, _: chrono::NaiveDateTime) -> Result<u64, DomainError> {
        panic!()
    }
}

// --- Single state factory — only auth_service varies ---

pub fn make_test_state(auth_service: Arc<dyn AuthService>) -> crate::state::AppState {
    let repo = Arc::new(Panic);
    crate::state::AppState {
        app_ctx: AppContext {
            repos: Repositories {
                movie: Arc::clone(&repo) as _,
                review: Arc::clone(&repo) as _,
                diary: Arc::clone(&repo) as _,
                stats: Arc::clone(&repo) as _,
                user: Arc::clone(&repo) as _,
                import_session: Arc::clone(&repo) as _,
                import_profile: Arc::clone(&repo) as _,
                movie_profile: Arc::clone(&repo) as _,
                watchlist: Arc::clone(&repo) as _,
                watch_event: Arc::clone(&repo) as _,
                webhook_token: Arc::clone(&repo) as _,
                profile_fields: Arc::clone(&repo) as _,
                person_command: Arc::clone(&repo) as _,
                person_query: Arc::clone(&repo) as _,
                search_port: Arc::clone(&repo) as _,
                search_command: Arc::clone(&repo) as _,
                remote_watchlist: Arc::clone(&repo) as _,
                social_query: Arc::clone(&repo) as _,
                wrapup_stats: Arc::clone(&repo) as _,
                wrapup_repo: Arc::clone(&repo) as _,
            },
            services: Services {
                auth: auth_service,
                password_hasher: Arc::clone(&repo) as _,
                metadata: Arc::clone(&repo) as _,
                poster_fetcher: Arc::clone(&repo) as _,
                image_storage: Arc::clone(&repo) as _,
                event_publisher: Arc::clone(&repo) as _,
                diary_exporter: Arc::clone(&repo) as _,
                document_parser: Arc::clone(&repo) as _,
                video_renderer: None,
            },
            config: AppConfig {
                allow_registration: false,
                base_url: "http://localhost:3000".to_string(),
                rate_limit: 20,
                wrapup: application::config::WrapUpConfig {
                    font_path: None,
                    logo_path: None,
                    bg_dir: None,
                    ffmpeg_path: "ffmpeg".into(),
                    max_concurrent_renders: 2,
                },
            },
        },
        rss_renderer: Arc::new(Panic),
        #[cfg(feature = "federation")]
        ap_service: Arc::new(activitypub::NoopActivityPubService),
        #[cfg(feature = "federation")]
        social_query: Arc::new(Panic),
    }
}

// --- Routers ---

async fn protected_handler(user: AuthenticatedUser) -> String {
    user.0.value().to_string()
}
async fn optional_cookie_handler(user: OptionalCookieUser) -> String {
    match user.0 {
        Some(id) => id.value().to_string(),
        None => "none".to_string(),
    }
}
async fn required_cookie_handler(user: RequiredCookieUser) -> String {
    user.0.value().to_string()
}

fn router_protected(state: crate::state::AppState) -> Router {
    Router::new()
        .route("/protected", get(protected_handler))
        .with_state(state)
}
fn router_optional(state: crate::state::AppState) -> Router {
    Router::new()
        .route("/optional", get(optional_cookie_handler))
        .with_state(state)
}
fn router_required(state: crate::state::AppState) -> Router {
    Router::new()
        .route("/required", get(required_cookie_handler))
        .with_state(state)
}

// --- Tests ---

#[tokio::test]
async fn missing_auth_header_returns_401() {
    let app = router_protected(make_test_state(Arc::new(Panic)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn optional_cookie_user_returns_none_without_cookie() {
    let app = router_optional(make_test_state(Arc::new(Panic)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/optional")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn optional_cookie_user_returns_none_with_invalid_token() {
    let app = router_optional(make_test_state(Arc::new(RejectingAuth)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/optional")
                .header("cookie", "token=bad.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"none");
}

#[tokio::test]
async fn required_cookie_user_redirects_without_cookie() {
    let app = router_required(make_test_state(Arc::new(Panic)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/required")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn required_cookie_user_redirects_with_invalid_token() {
    let app = router_required(make_test_state(Arc::new(RejectingAuth)));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/required")
                .header("cookie", "token=bad.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}
