use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa_scalar::{Scalar, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub trait ApiDocExt {
    fn with_api_doc(self, spec: OpenApi) -> Self;
}

impl ApiDocExt for Router {
    fn with_api_doc(self, spec: OpenApi) -> Self {
        tracing::info!("API docs at /docs (Swagger) and /scalar");
        self.merge(SwaggerUi::new("/docs").url("/openapi.json", spec.clone()))
            .merge(Scalar::with_url("/scalar", spec))
    }
}
