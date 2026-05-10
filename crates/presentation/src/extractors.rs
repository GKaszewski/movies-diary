use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Redirect},
};
use domain::{errors::DomainError, value_objects::UserId};

use crate::{errors::ApiError, state::AppState};

pub struct AuthenticatedUser(pub UserId);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                ApiError(DomainError::Unauthorized(
                    "Missing or invalid auth token".into(),
                ))
            })?;
        let user_id = app_state.app_ctx.auth_service.validate_token(token).await?;
        Ok(AuthenticatedUser(user_id))
    }
}

pub struct OptionalCookieUser(pub Option<UserId>);
pub struct RequiredCookieUser(pub UserId);

fn extract_token_from_cookie(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find_map(|c| c.trim().strip_prefix("token=").map(str::to_string))
        })
}

impl<S> FromRequestParts<S> for OptionalCookieUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let Some(token) = extract_token_from_cookie(parts) else {
            return Ok(OptionalCookieUser(None));
        };
        let user_id = app_state
            .app_ctx
            .auth_service
            .validate_token(&token)
            .await
            .ok();
        Ok(OptionalCookieUser(user_id))
    }
}

impl<S> FromRequestParts<S> for RequiredCookieUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_token_from_cookie(parts)
            .ok_or_else(|| Redirect::to("/login").into_response())?;
        let user_id = app_state
            .app_ctx
            .auth_service
            .validate_token(&token)
            .await
            .map_err(|_| Redirect::to("/login").into_response())?;
        Ok(RequiredCookieUser(user_id))
    }
}

pub struct AdminUser(pub UserId);

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let RequiredCookieUser(user_id) =
            RequiredCookieUser::from_request_parts(parts, state).await?;
        let user = app_state
            .app_ctx
            .user_repository
            .find_by_id(&user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())?;
        match user.role() {
            domain::models::UserRole::Admin => Ok(AdminUser(user_id)),
            _ => Err(StatusCode::FORBIDDEN.into_response()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::{config::AppConfig, context::AppContext};
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
            DiaryEntry, DiaryFilter, FeedEntry, Movie, Review, ReviewHistory, UserStats,
            UserTrends,
            collections::{PageParams, Paginated},
        },
        ports::{
            AuthService, DiaryRepository, EventPublisher, GeneratedToken, MetadataClient,
            MovieRepository, PasswordHasher, PosterFetcherClient, PosterStorage, ReviewRepository,
            StatsRepository, UserRepository,
        },
        value_objects::{
            Email, ExternalMetadataId, MovieId, MovieTitle, PasswordHash, PosterPath, PosterUrl,
            ReleaseYear, ReviewId, UserId,
        },
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    // --- Panic stubs (defined once) ---

    struct Panic;

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
    }
    #[cfg(feature = "federation")]
    #[async_trait::async_trait]
    impl domain::ports::SocialQueryPort for Panic {
        async fn get_accepted_following_urls(
            &self,
            _: uuid::Uuid,
        ) -> Result<Vec<String>, DomainError> {
            panic!()
        }
        async fn list_all_followed_remote_actors(
            &self,
        ) -> Result<Vec<domain::ports::RemoteActorInfo>, DomainError> {
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
    impl PosterStorage for Panic {
        async fn store_poster(&self, _: &MovieId, _: &[u8]) -> Result<PosterPath, DomainError> {
            panic!()
        }
        async fn get_poster(&self, _: &PosterPath) -> Result<Vec<u8>, DomainError> {
            panic!()
        }
        async fn delete_poster(&self, _: &PosterPath) -> Result<(), DomainError> {
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
        async fn find_by_email(
            &self,
            _: &Email,
        ) -> Result<Option<domain::models::User>, DomainError> {
            panic!()
        }
        async fn save(&self, _: &domain::models::User) -> Result<(), DomainError> {
            panic!()
        }
        async fn find_by_id(
            &self,
            _: &UserId,
        ) -> Result<Option<domain::models::User>, DomainError> {
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
    }
    #[async_trait::async_trait]
    impl EventPublisher for Panic {
        async fn publish(&self, _: &DomainEvent) -> Result<(), DomainError> {
            panic!()
        }
    }
    #[async_trait::async_trait]
    impl domain::ports::ImportSessionRepository for Panic {
        async fn create(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> { panic!() }
        async fn get(&self, _: &domain::value_objects::ImportSessionId, _: &UserId) -> Result<Option<domain::models::ImportSession>, DomainError> { panic!() }
        async fn update(&self, _: &domain::models::ImportSession) -> Result<(), DomainError> { panic!() }
        async fn delete(&self, _: &domain::value_objects::ImportSessionId) -> Result<(), DomainError> { panic!() }
        async fn delete_expired(&self) -> Result<u64, DomainError> { panic!() }
        async fn delete_expired_for_user(&self, _: &UserId) -> Result<(), DomainError> { panic!() }
    }
    #[async_trait::async_trait]
    impl domain::ports::ImportProfileRepository for Panic {
        async fn save(&self, _: &domain::models::ImportProfile) -> Result<(), DomainError> { panic!() }
        async fn list_for_user(&self, _: &UserId) -> Result<Vec<domain::models::ImportProfile>, DomainError> { panic!() }
        async fn get(&self, _: &domain::value_objects::ImportProfileId, _: &UserId) -> Result<Option<domain::models::ImportProfile>, DomainError> { panic!() }
        async fn delete(&self, _: &domain::value_objects::ImportProfileId) -> Result<(), DomainError> { panic!() }
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
        fn parse(&self, _: &[u8], _: domain::models::FileFormat) -> Result<domain::models::ParsedFile, domain::models::ImportError> {
            panic!()
        }
        fn apply_mapping(&self, _: &domain::models::ParsedFile, _: &[domain::models::FieldMapping]) -> Vec<domain::models::AnnotatedRow> {
            panic!()
        }
    }

    impl crate::ports::HtmlRenderer for Panic {
        fn render_diary_page(
            &self,
            _: &Paginated<DiaryEntry>,
            _: application::ports::HtmlPageContext,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_login_page(
            &self,
            _: application::ports::LoginPageData<'_>,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_register_page(
            &self,
            _: application::ports::RegisterPageData<'_>,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_new_review_page(
            &self,
            _: application::ports::NewReviewPageData<'_>,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_activity_feed_page(
            &self,
            _: application::ports::ActivityFeedPageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_users_page(
            &self,
            _: application::ports::UsersPageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_profile_page(
            &self,
            _: application::ports::ProfilePageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_following_page(
            &self,
            _: application::ports::FollowingPageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_followers_page(
            &self,
            _: application::ports::FollowersPageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_movie_detail_page(
            &self,
            _: application::ports::MovieDetailPageData,
        ) -> Result<String, String> {
            panic!()
        }
        fn render_import_upload_page(&self, _: application::ports::ImportUploadPageData) -> Result<String, String> { panic!() }
        fn render_import_mapping_page(&self, _: application::ports::ImportMappingPageData) -> Result<String, String> { panic!() }
        fn render_import_preview_page(&self, _: application::ports::ImportPreviewPageData) -> Result<String, String> { panic!() }
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

    // --- Single state factory — only auth_service varies ---

    fn make_test_state(auth_service: Arc<dyn AuthService>) -> crate::state::AppState {
        let repo = Arc::new(Panic);
        crate::state::AppState {
            app_ctx: AppContext {
                movie_repository: Arc::clone(&repo) as _,
                review_repository: Arc::clone(&repo) as _,
                diary_repository: Arc::clone(&repo) as _,
                diary_exporter: Arc::clone(&repo) as _,
                document_parser: Arc::clone(&repo) as _,
                stats_repository: Arc::clone(&repo) as _,
                metadata_client: Arc::clone(&repo) as _,
                poster_fetcher: Arc::clone(&repo) as _,
                poster_storage: Arc::clone(&repo) as _,
                event_publisher: Arc::clone(&repo) as _,
                password_hasher: Arc::clone(&repo) as _,
                user_repository: Arc::clone(&repo) as _,
                import_session_repository: Arc::clone(&repo) as _,
                import_profile_repository: Arc::clone(&repo) as _,
                auth_service,
                config: AppConfig {
                    allow_registration: false,
                    base_url: "http://localhost:3000".to_string(),
                    rate_limit: 20,
                },
            },
            html_renderer: Arc::new(Panic),
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
}
