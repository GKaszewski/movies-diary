use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
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
                ApiError(DomainError::ValidationError(
                    "Missing auth token".into(),
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
    async fn missing_auth_header_returns_400() {
        use std::sync::Arc;
        use application::context::AppContext;
        use auth::StubAuthService;

        struct PanicRepo;
        #[async_trait::async_trait]
        impl domain::ports::MovieRepository for PanicRepo {
            async fn get_movie_by_external_id(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movie_by_id(&self, _: &domain::value_objects::MovieId) -> Result<Option<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn get_movies_by_title_and_year(&self, _: &domain::value_objects::MovieTitle, _: &domain::value_objects::ReleaseYear) -> Result<Vec<domain::models::Movie>, domain::errors::DomainError> { panic!() }
            async fn upsert_movie(&self, _: &domain::models::Movie) -> Result<(), domain::errors::DomainError> { panic!() }
            async fn save_review(&self, _: &domain::models::Review) -> Result<domain::events::DomainEvent, domain::errors::DomainError> { panic!() }
            async fn query_diary(&self, _: &domain::models::DiaryFilter) -> Result<domain::models::collections::Paginated<domain::models::DiaryEntry>, domain::errors::DomainError> { panic!() }
            async fn get_review_history(&self, _: &domain::value_objects::MovieId) -> Result<domain::models::ReviewHistory, domain::errors::DomainError> { panic!() }
        }

        struct PanicRenderer;
        impl crate::ports::HtmlRenderer for PanicRenderer {
            fn render_diary_page(&self, _: &domain::models::collections::Paginated<domain::models::DiaryEntry>) -> Result<String, String> { panic!() }
        }

        struct PanicMeta; struct PanicFetcher; struct PanicStorage; struct PanicEvent; struct PanicHasher;
        #[async_trait::async_trait] impl domain::ports::MetadataClient for PanicMeta { async fn fetch_movie_metadata(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<domain::models::Movie, domain::errors::DomainError> { panic!() } async fn get_poster_url(&self, _: &domain::value_objects::ExternalMetadataId) -> Result<Option<domain::value_objects::PosterUrl>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterFetcherClient for PanicFetcher { async fn fetch_poster_bytes(&self, _: &domain::value_objects::PosterUrl) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PosterStorage for PanicStorage { async fn store_poster(&self, _: &domain::value_objects::MovieId, _: &[u8]) -> Result<domain::value_objects::PosterPath, domain::errors::DomainError> { panic!() } async fn get_poster(&self, _: &domain::value_objects::PosterPath) -> Result<Vec<u8>, domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::EventPublisher for PanicEvent { async fn publish(&self, _: &domain::events::DomainEvent) -> Result<(), domain::errors::DomainError> { panic!() } }
        #[async_trait::async_trait] impl domain::ports::PasswordHasher for PanicHasher { async fn hash(&self, _: &str) -> Result<domain::value_objects::PasswordHash, domain::errors::DomainError> { panic!() } async fn verify(&self, _: &str, _: &domain::value_objects::PasswordHash) -> Result<bool, domain::errors::DomainError> { panic!() } }

        let state = crate::state::AppState {
            app_ctx: AppContext {
                repository: Arc::new(PanicRepo),
                metadata_client: Arc::new(PanicMeta),
                poster_fetcher: Arc::new(PanicFetcher),
                poster_storage: Arc::new(PanicStorage),
                event_publisher: Arc::new(PanicEvent),
                auth_service: Arc::new(StubAuthService),
                password_hasher: Arc::new(PanicHasher),
            },
            html_renderer: Arc::new(PanicRenderer),
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
