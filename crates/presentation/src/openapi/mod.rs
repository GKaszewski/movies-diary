mod auth;
mod diary;
mod import;
mod movies;
mod social;
mod users;

use axum::Router;
use domain::PRODUCT_NAME;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};
use utoipa_scalar::{Scalar, Servable};
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

fn build() -> utoipa::openapi::OpenApi {
    let mut api = auth::AuthDoc::openapi();
    api.info = utoipa::openapi::InfoBuilder::new()
        .title(format!("{} API", PRODUCT_NAME))
        .version("1.0.0")
        .description(Some(format!("REST API for {}. Authenticate with `POST /api/v1/auth/login` to get a Bearer token.", PRODUCT_NAME)))
        .build();
    api.merge(diary::DiaryDoc::openapi());
    api.merge(movies::MoviesDoc::openapi());
    api.merge(users::UsersDoc::openapi());
    api.merge(import::ImportDoc::openapi());
    #[cfg(feature = "federation")]
    api.merge(social::SocialDoc::openapi());
    SecurityAddon.modify(&mut api);
    api
}

pub fn serve(router: Router) -> Router {
    tracing::info!("API docs at /docs (Swagger) and /scalar");
    let spec = build();
    router
        .merge(SwaggerUi::new("/docs").url("/openapi.json", spec.clone()))
        .merge(Scalar::with_url("/scalar", spec))
}
