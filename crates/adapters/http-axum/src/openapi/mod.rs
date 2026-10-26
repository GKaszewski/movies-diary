mod auth;
mod diary;
mod goals;
mod import;
mod insights;
mod movies;
mod search;
mod social;
mod users;
mod watchlist;
mod webhook;
mod wrapup;

use axum::Router;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

pub fn spec() -> utoipa::openapi::OpenApi {
    let mut api = auth::AuthDoc::openapi();
    api.components
        .get_or_insert_with(Default::default)
        .schemas
        .insert(
            "ErrorResponse".into(),
            <api_types::ErrorResponse as utoipa::PartialSchema>::schema(),
        );
    api.info = utoipa::openapi::InfoBuilder::new()
        .title("Movies Diary API")
        .version("1.0.0")
        .description(Some("REST API for Movies Diary. Authenticate with `POST /api/v1/auth/login` to get a Bearer token."))
        .build();
    api.merge(diary::DiaryDoc::openapi());
    api.merge(movies::MoviesDoc::openapi());
    api.merge(users::UsersDoc::openapi());
    api.merge(import::ImportDoc::openapi());
    api.merge(search::SearchDoc::openapi());
    api.merge(watchlist::WatchlistDoc::openapi());
    api.merge(goals::GoalsDoc::openapi());
    api.merge(webhook::WebhookDoc::openapi());
    api.merge(wrapup::WrapUpDoc::openapi());
    api.merge(insights::InsightsDoc::openapi());
    #[cfg(feature = "federation")]
    api.merge(social::SocialDoc::openapi());
    SecurityAddon.modify(&mut api);
    api
}

pub fn serve(router: Router) -> Router {
    tracing::info!("API docs at /docs (Swagger)");
    let spec = spec();
    router.merge(SwaggerUi::new("/docs").url("/openapi.json", spec))
}
