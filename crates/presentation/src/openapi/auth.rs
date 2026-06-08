use api_types::{LoginRequest, LoginResponse, RegisterRequest};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(crate::handlers::auth::login, crate::handlers::auth::register,),
    components(schemas(LoginRequest, LoginResponse, RegisterRequest))
)]
pub struct AuthDoc;
