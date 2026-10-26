use api_types::{
    ApiFieldMapping, ApplyMappingRequest, ConfirmRequest, SaveProfileRequest,
    SessionCreatedResponse, SessionStateResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::import::api_post_session,
        crate::api::import::api_get_session,
        crate::api::import::api_put_mapping,
        crate::api::import::api_post_confirm,
        crate::api::import::api_get_profiles,
        crate::api::import::api_post_profile,
        crate::api::import::api_delete_profile,
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
