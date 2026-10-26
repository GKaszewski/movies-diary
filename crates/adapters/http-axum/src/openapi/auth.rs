use api_types::{LoginRequest, LoginResponse, RegisterRequest};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(crate::api::auth::login, crate::api::auth::register,),
    components(schemas(LoginRequest, LoginResponse, RegisterRequest))
)]
pub struct AuthDoc;
