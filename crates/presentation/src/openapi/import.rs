use api_types::{
    ApiFieldMapping, ApplyMappingRequest, ConfirmRequest, SaveProfileRequest,
    SessionCreatedResponse, SessionStateResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::import::api_post_session,
        crate::handlers::import::api_get_session,
        crate::handlers::import::api_put_mapping,
        crate::handlers::import::api_post_confirm,
        crate::handlers::import::api_get_profiles,
        crate::handlers::import::api_post_profile,
        crate::handlers::import::api_delete_profile,
    ),
    components(schemas(
        SessionCreatedResponse,
        SessionStateResponse,
        ApiFieldMapping,
        ApplyMappingRequest,
        ConfirmRequest,
        SaveProfileRequest,
    ))
)]
pub struct ImportDoc;
