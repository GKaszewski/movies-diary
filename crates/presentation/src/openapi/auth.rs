use api_types::{LoginRequest, LoginResponse, RegisterRequest};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::login,
        crate::handlers::api::register,
    ),
    components(schemas(LoginRequest, LoginResponse, RegisterRequest)),
)]
pub struct AuthDoc;
