use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, header::AUTHORIZATION, request::Parts},
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
        let user_id = app_state
            .app_ctx
            .auth_service
            .validate_token(token)
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn protected_handler(user: AuthenticatedUser) -> String {
        user.0.value().to_string()
    }

    fn test_router(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/protected", get(protected_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn missing_auth_header_returns_401() {
        use std::sync::Arc;
        use application::context::AppContext;

        struct PanicRepo;
        #[async_trait::async_trait]
        impl domain::ports::MovieRepository for PanicRepo {
            async fn get_movie_by_external_id(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movie_by_id(&self, _: &domain::value_objects::MovieId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movies_by_title_and_year(&self, _: &domain::value_objects::MovieTitle, _: &domain::value_objects::ReleaseYear) -> Result<Vec<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn upsert_movie(&self, _: &domain::models::Movie) -> Result<(), domain::errors::DomainError> { panic!() }
            async fn delete_movie(&self, _: &domain::value_objects::MovieId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::ReviewRepository for PanicRepo {
            async fn save_review(&self, _: &domain::models::Review) -> Result<domain::events::DomainEvent, domain::errors::DomainError> { panic!() }
            async fn get_review_by_id(&self, _: &domain::value_objects::ReviewId) -> Result<Option<domain::models::Review>, domain::errors::DomainError> { panic!() }
            async fn delete_review(&self, _: &domain::value_objects::ReviewId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::DiaryRepository for PanicRepo {
            async fn query_diary(&self, _: &domain::models::DiaryFilter) -> Result<domain::models::collections::Paginated<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
            async fn query_activity_feed(&self, _: &domain::models::collections::PageParams) -> Result<domain::models::collections::Paginated<domain::models::FeedEntry>, domain::errors::DomainError> { panic!() }
            async fn get_review_history(&self, _: &domain::value_objects::MovieId) -> Result<domain::models::ReviewHistory, domain::errors::DomainError> { panic!() }
            async fn get_user_history(&self, _: &domain::value_objects::UserId) -> Result<Vec<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::StatsRepository for PanicRepo {
            async fn get_user_stats(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserStats, domain::errors::DomainError> { panic!() }
            async fn get_user_trends(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserTrends, domain::errors::DomainError> { panic!() }
        }

        struct PanicRenderer;
        impl crate::ports::HtmlRenderer for PanicRenderer {
            fn render_diary_page(&self, _: &domain::models::collections::Paginated<domain::models::DiaryEntry>, _: application::ports::HtmlPageContext) -> Result<String, String> { panic!() }
            fn render_login_page(&self, _: application::ports::LoginPageData<'_>) -> Result<String, String> { panic!() }
            fn render_register_page(&self, _: application::ports::RegisterPageData<'_>) -> Result<String, String> { panic!() }
            fn render_new_review_page(&self, _: application::ports::NewReviewPageData<'_>) -> Result<String, String> { panic!() }
            fn render_activity_feed_page(&self, _: application::ports::ActivityFeedPageData) -> Result<String, String> { panic!() }
            fn render_users_page(&self, _: application::ports::UsersPageData) -> Result<String, String> { panic!() }
            fn render_profile_page(&self, _: application::ports::ProfilePageData) -> Result<String, String> { panic!() }
            fn render_following_page(&self, _: application::ports::FollowingPageData) -> Result<String, String> { panic!() }
            fn render_followers_page(&self, _: application::ports::FollowersPageData) -> Result<String, String> { panic!() }
        }

        struct PanicRssRenderer;
        impl crate::ports::RssFeedRenderer for PanicRssRenderer {
            fn render_feed(&self, _: &[domain::models::DiaryEntry], _: &str) -> Result<String, String> { panic!() }
        }

        struct PanicMeta; struct PanicFetcher; struct PanicStorage; struct PanicEvent; struct PanicHasher; struct PanicAuth; struct PanicUserRepo;
        #[async_trait::async_trait] impl domain::ports::MetadataClient for PanicMeta { async fn fetch_movie_metadata(&self, _: &domain::ports::MetadataSearchCriteria) -> Result<domain::models::Movie, domain::errors::DomainError> { panic!() } async fn get_poster_url(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::value_objects::PosterUrl>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterFetcherClient for PanicFetcher { async fn fetch_poster_bytes(&self, _: &domain::value_objects::PosterUrl) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterStorage for PanicStorage { async fn store_poster(&self, _: &domain::value_objects::MovieId, _: &[u8]) -> Result<domain::value_objects::PosterPath, domain::errors::DomainError> { panic!() } async fn get_poster(&self, _: &domain::value_objects::PosterPath) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::EventPublisher for PanicEvent { async fn publish(&self, _: &domain::events::DomainEvent) -> Result<(), domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PasswordHasher for PanicHasher { async fn hash(&self, _: &str) -> Result<domain::value_objects::PasswordHash, domain::errors::DomainError> { panic!() } async fn verify(&self, _: &str, _: &domain::value_objects::PasswordHash) -> Result<bool, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::AuthService for PanicAuth { async fn generate_token(&self, _: &domain::value_objects::UserId) -> Result<domain::ports::GeneratedToken, domain::errors::DomainError> { panic!() } async fn validate_token(&self, _: &str) -> Result<domain::value_objects::UserId, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::UserRepository for PanicUserRepo { async fn find_by_email(&self, _: &domain::value_objects::Email) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn save(&self, _: &domain::models::User) -> Result<(), domain::errors::DomainError> { panic!() } async fn find_by_id(&self, _: &domain::value_objects::UserId) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn find_by_username(&self, _: &domain::value_objects::Username) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, domain::errors::DomainError> { panic!() } }

        let state = crate::state::AppState {
            app_ctx: AppContext {
                movie_repository: Arc::new(PanicRepo) as _,
                review_repository: Arc::new(PanicRepo) as _,
                diary_repository: Arc::new(PanicRepo) as _,
                stats_repository: Arc::new(PanicRepo) as _,
                metadata_client: Arc::new(PanicMeta),
                poster_fetcher: Arc::new(PanicFetcher),
                poster_storage: Arc::new(PanicStorage),
                event_publisher: Arc::new(PanicEvent),
                auth_service: Arc::new(PanicAuth),
                password_hasher: Arc::new(PanicHasher),
                user_repository: Arc::new(PanicUserRepo),
                config: application::config::AppConfig { allow_registration: false, base_url: "http://localhost:3000".to_string(), rate_limit: 20 },
            },
            html_renderer: Arc::new(PanicRenderer),
            rss_renderer: Arc::new(PanicRssRenderer),
            ap_service: std::sync::Arc::new(activitypub::NoopActivityPubService),
        };

        let app = test_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // Reusable helpers for cookie extractor tests
    async fn optional_cookie_handler(user: OptionalCookieUser) -> String {
        match user.0 {
            Some(id) => id.value().to_string(),
            None => "none".to_string(),
        }
    }

    async fn required_cookie_handler(user: RequiredCookieUser) -> String {
        user.0.value().to_string()
    }

    fn test_router_optional(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/optional", get(optional_cookie_handler))
            .with_state(state)
    }

    fn test_router_required(state: crate::state::AppState) -> Router {
        Router::new()
            .route("/required", get(required_cookie_handler))
            .with_state(state)
    }

    struct RejectingAuth;
    #[async_trait::async_trait]
    impl domain::ports::AuthService for RejectingAuth {
        async fn generate_token(&self, _: &domain::value_objects::UserId) -> Result<domain::ports::GeneratedToken, domain::errors::DomainError> { panic!() }
        async fn validate_token(&self, _: &str) -> Result<domain::value_objects::UserId, domain::errors::DomainError> {
            Err(domain::errors::DomainError::Unauthorized("bad token".into()))
        }
    }

    async fn panic_state() -> crate::state::AppState {
        use std::sync::Arc;
        use application::context::AppContext;
        struct PanicRepo2;
        #[async_trait::async_trait]
        impl domain::ports::MovieRepository for PanicRepo2 {
            async fn get_movie_by_external_id(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movie_by_id(&self, _: &domain::value_objects::MovieId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movies_by_title_and_year(&self, _: &domain::value_objects::MovieTitle, _: &domain::value_objects::ReleaseYear) -> Result<Vec<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn upsert_movie(&self, _: &domain::models::Movie) -> Result<(), domain::errors::DomainError> { panic!() }
            async fn delete_movie(&self, _: &domain::value_objects::MovieId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::ReviewRepository for PanicRepo2 {
            async fn save_review(&self, _: &domain::models::Review) -> Result<domain::events::DomainEvent, domain::errors::DomainError> { panic!() }
            async fn get_review_by_id(&self, _: &domain::value_objects::ReviewId) -> Result<Option<domain::models::Review>, domain::errors::DomainError> { panic!() }
            async fn delete_review(&self, _: &domain::value_objects::ReviewId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::DiaryRepository for PanicRepo2 {
            async fn query_diary(&self, _: &domain::models::DiaryFilter) -> Result<domain::models::collections::Paginated<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
            async fn query_activity_feed(&self, _: &domain::models::collections::PageParams) -> Result<domain::models::collections::Paginated<domain::models::FeedEntry>, domain::errors::DomainError> { panic!() }
            async fn get_review_history(&self, _: &domain::value_objects::MovieId) -> Result<domain::models::ReviewHistory, domain::errors::DomainError> { panic!() }
            async fn get_user_history(&self, _: &domain::value_objects::UserId) -> Result<Vec<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::StatsRepository for PanicRepo2 {
            async fn get_user_stats(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserStats, domain::errors::DomainError> { panic!() }
            async fn get_user_trends(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserTrends, domain::errors::DomainError> { panic!() }
        }
        struct PanicMeta2; struct PanicFetcher2; struct PanicStorage2; struct PanicEvent2; struct PanicHasher2; struct PanicUserRepo2;
        #[async_trait::async_trait] impl domain::ports::MetadataClient for PanicMeta2 { async fn fetch_movie_metadata(&self, _: &domain::ports::MetadataSearchCriteria) -> Result<domain::models::Movie, domain::errors::DomainError> { panic!() } async fn get_poster_url(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::value_objects::PosterUrl>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterFetcherClient for PanicFetcher2 { async fn fetch_poster_bytes(&self, _: &domain::value_objects::PosterUrl) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterStorage for PanicStorage2 { async fn store_poster(&self, _: &domain::value_objects::MovieId, _: &[u8]) -> Result<domain::value_objects::PosterPath, domain::errors::DomainError> { panic!() } async fn get_poster(&self, _: &domain::value_objects::PosterPath) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::EventPublisher for PanicEvent2 { async fn publish(&self, _: &domain::events::DomainEvent) -> Result<(), domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PasswordHasher for PanicHasher2 { async fn hash(&self, _: &str) -> Result<domain::value_objects::PasswordHash, domain::errors::DomainError> { panic!() } async fn verify(&self, _: &str, _: &domain::value_objects::PasswordHash) -> Result<bool, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::AuthService for PanicAuth2 { async fn generate_token(&self, _: &domain::value_objects::UserId) -> Result<domain::ports::GeneratedToken, domain::errors::DomainError> { panic!() } async fn validate_token(&self, _: &str) -> Result<domain::value_objects::UserId, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::UserRepository for PanicUserRepo2 { async fn find_by_email(&self, _: &domain::value_objects::Email) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn save(&self, _: &domain::models::User) -> Result<(), domain::errors::DomainError> { panic!() } async fn find_by_id(&self, _: &domain::value_objects::UserId) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn find_by_username(&self, _: &domain::value_objects::Username) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, domain::errors::DomainError> { panic!() } }
        struct PanicRenderer2;
        impl crate::ports::HtmlRenderer for PanicRenderer2 {
            fn render_diary_page(&self, _: &domain::models::collections::Paginated<domain::models::DiaryEntry>, _: application::ports::HtmlPageContext) -> Result<String, String> { panic!() }
            fn render_login_page(&self, _: application::ports::LoginPageData<'_>) -> Result<String, String> { panic!() }
            fn render_register_page(&self, _: application::ports::RegisterPageData<'_>) -> Result<String, String> { panic!() }
            fn render_new_review_page(&self, _: application::ports::NewReviewPageData<'_>) -> Result<String, String> { panic!() }
            fn render_activity_feed_page(&self, _: application::ports::ActivityFeedPageData) -> Result<String, String> { panic!() }
            fn render_users_page(&self, _: application::ports::UsersPageData) -> Result<String, String> { panic!() }
            fn render_profile_page(&self, _: application::ports::ProfilePageData) -> Result<String, String> { panic!() }
            fn render_following_page(&self, _: application::ports::FollowingPageData) -> Result<String, String> { panic!() }
            fn render_followers_page(&self, _: application::ports::FollowersPageData) -> Result<String, String> { panic!() }
        }
        struct PanicRssRenderer2;
        impl crate::ports::RssFeedRenderer for PanicRssRenderer2 {
            fn render_feed(&self, _: &[domain::models::DiaryEntry], _: &str) -> Result<String, String> { panic!() }
        }
        struct PanicAuth2;
        crate::state::AppState {
            app_ctx: AppContext {
                movie_repository: Arc::new(PanicRepo2) as _,
                review_repository: Arc::new(PanicRepo2) as _,
                diary_repository: Arc::new(PanicRepo2) as _,
                stats_repository: Arc::new(PanicRepo2) as _,
                metadata_client: Arc::new(PanicMeta2),
                poster_fetcher: Arc::new(PanicFetcher2),
                poster_storage: Arc::new(PanicStorage2),
                event_publisher: Arc::new(PanicEvent2),
                auth_service: Arc::new(PanicAuth2),
                password_hasher: Arc::new(PanicHasher2),
                user_repository: Arc::new(PanicUserRepo2),
                config: application::config::AppConfig { allow_registration: false, base_url: "http://localhost:3000".to_string(), rate_limit: 20 },
            },
            html_renderer: Arc::new(PanicRenderer2),
            rss_renderer: Arc::new(PanicRssRenderer2),
            ap_service: std::sync::Arc::new(activitypub::NoopActivityPubService),
        }
    }

    async fn rejecting_state() -> crate::state::AppState {
        use std::sync::Arc;
        use application::context::AppContext;
        struct PanicRepo3;
        #[async_trait::async_trait]
        impl domain::ports::MovieRepository for PanicRepo3 {
            async fn get_movie_by_external_id(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movie_by_id(&self, _: &domain::value_objects::MovieId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movies_by_title_and_year(&self, _: &domain::value_objects::MovieTitle, _: &domain::value_objects::ReleaseYear) -> Result<Vec<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn upsert_movie(&self, _: &domain::models::Movie) -> Result<(), domain::errors::DomainError> { panic!() }
            async fn delete_movie(&self, _: &domain::value_objects::MovieId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::ReviewRepository for PanicRepo3 {
            async fn save_review(&self, _: &domain::models::Review) -> Result<domain::events::DomainEvent, domain::errors::DomainError> { panic!() }
            async fn get_review_by_id(&self, _: &domain::value_objects::ReviewId) -> Result<Option<domain::models::Review>, domain::errors::DomainError> { panic!() }
            async fn delete_review(&self, _: &domain::value_objects::ReviewId) -> Result<(), domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::DiaryRepository for PanicRepo3 {
            async fn query_diary(&self, _: &domain::models::DiaryFilter) -> Result<domain::models::collections::Paginated<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
            async fn query_activity_feed(&self, _: &domain::models::collections::PageParams) -> Result<domain::models::collections::Paginated<domain::models::FeedEntry>, domain::errors::DomainError> { panic!() }
            async fn get_review_history(&self, _: &domain::value_objects::MovieId) -> Result<domain::models::ReviewHistory, domain::errors::DomainError> { panic!() }
            async fn get_user_history(&self, _: &domain::value_objects::UserId) -> Result<Vec<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
        }
        #[async_trait::async_trait]
        impl domain::ports::StatsRepository for PanicRepo3 {
            async fn get_user_stats(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserStats, domain::errors::DomainError> { panic!() }
            async fn get_user_trends(&self, _: &domain::value_objects::UserId) -> Result<domain::models::UserTrends, domain::errors::DomainError> { panic!() }
        }
        struct PanicMeta3; struct PanicFetcher3; struct PanicStorage3; struct PanicEvent3; struct PanicHasher3; struct PanicUserRepo3;
        #[async_trait::async_trait] impl domain::ports::MetadataClient for PanicMeta3 { async fn fetch_movie_metadata(&self, _: &domain::ports::MetadataSearchCriteria) -> Result<domain::models::Movie, domain::errors::DomainError> { panic!() } async fn get_poster_url(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::value_objects::PosterUrl>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterFetcherClient for PanicFetcher3 { async fn fetch_poster_bytes(&self, _: &domain::value_objects::PosterUrl) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterStorage for PanicStorage3 { async fn store_poster(&self, _: &domain::value_objects::MovieId, _: &[u8]) -> Result<domain::value_objects::PosterPath, domain::errors::DomainError> { panic!() } async fn get_poster(&self, _: &domain::value_objects::PosterPath) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::EventPublisher for PanicEvent3 { async fn publish(&self, _: &domain::events::DomainEvent) -> Result<(), domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PasswordHasher for PanicHasher3 { async fn hash(&self, _: &str) -> Result<domain::value_objects::PasswordHash, domain::errors::DomainError> { panic!() } async fn verify(&self, _: &str, _: &domain::value_objects::PasswordHash) -> Result<bool, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::UserRepository for PanicUserRepo3 { async fn find_by_email(&self, _: &domain::value_objects::Email) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn save(&self, _: &domain::models::User) -> Result<(), domain::errors::DomainError> { panic!() } async fn find_by_id(&self, _: &domain::value_objects::UserId) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn find_by_username(&self, _: &domain::value_objects::Username) -> Result<Option<domain::models::User>, domain::errors::DomainError> { panic!() } async fn list_with_stats(&self) -> Result<Vec<domain::models::UserSummary>, domain::errors::DomainError> { panic!() } }
        struct PanicRenderer3;
        impl crate::ports::HtmlRenderer for PanicRenderer3 {
            fn render_diary_page(&self, _: &domain::models::collections::Paginated<domain::models::DiaryEntry>, _: application::ports::HtmlPageContext) -> Result<String, String> { panic!() }
            fn render_login_page(&self, _: application::ports::LoginPageData<'_>) -> Result<String, String> { panic!() }
            fn render_register_page(&self, _: application::ports::RegisterPageData<'_>) -> Result<String, String> { panic!() }
            fn render_new_review_page(&self, _: application::ports::NewReviewPageData<'_>) -> Result<String, String> { panic!() }
            fn render_activity_feed_page(&self, _: application::ports::ActivityFeedPageData) -> Result<String, String> { panic!() }
            fn render_users_page(&self, _: application::ports::UsersPageData) -> Result<String, String> { panic!() }
            fn render_profile_page(&self, _: application::ports::ProfilePageData) -> Result<String, String> { panic!() }
            fn render_following_page(&self, _: application::ports::FollowingPageData) -> Result<String, String> { panic!() }
            fn render_followers_page(&self, _: application::ports::FollowersPageData) -> Result<String, String> { panic!() }
        }
        struct PanicRssRenderer3;
        impl crate::ports::RssFeedRenderer for PanicRssRenderer3 {
            fn render_feed(&self, _: &[domain::models::DiaryEntry], _: &str) -> Result<String, String> { panic!() }
        }
        crate::state::AppState {
            app_ctx: AppContext {
                movie_repository: Arc::new(PanicRepo3) as _,
                review_repository: Arc::new(PanicRepo3) as _,
                diary_repository: Arc::new(PanicRepo3) as _,
                stats_repository: Arc::new(PanicRepo3) as _,
                metadata_client: Arc::new(PanicMeta3),
                poster_fetcher: Arc::new(PanicFetcher3),
                poster_storage: Arc::new(PanicStorage3),
                event_publisher: Arc::new(PanicEvent3),
                auth_service: Arc::new(RejectingAuth),
                password_hasher: Arc::new(PanicHasher3),
                user_repository: Arc::new(PanicUserRepo3),
                config: application::config::AppConfig { allow_registration: false, base_url: "http://localhost:3000".to_string(), rate_limit: 20 },
            },
            html_renderer: Arc::new(PanicRenderer3),
            rss_renderer: Arc::new(PanicRssRenderer3),
            ap_service: std::sync::Arc::new(activitypub::NoopActivityPubService),
        }
    }

    #[tokio::test]
    async fn optional_cookie_user_returns_none_without_cookie() {
        let app = test_router_optional(panic_state().await);
        let response = app
            .oneshot(Request::builder().uri("/optional").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"none");
    }

    #[tokio::test]
    async fn optional_cookie_user_returns_none_with_invalid_token() {
        let app = test_router_optional(rejecting_state().await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/optional")
                    .header("cookie", "token=bad.token.here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"none");
    }

    #[tokio::test]
    async fn required_cookie_user_redirects_without_cookie() {
        let app = test_router_required(panic_state().await);
        let response = app
            .oneshot(Request::builder().uri("/required").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/login");
    }

    #[tokio::test]
    async fn required_cookie_user_redirects_with_invalid_token() {
        let app = test_router_required(rejecting_state().await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/required")
                    .header("cookie", "token=bad.token.here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get("location").unwrap(), "/login");
    }
}
