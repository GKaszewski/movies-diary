use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::wrapup::post_generate,
        crate::api::wrapup::get_list,
        crate::api::wrapup::get_status,
        crate::api::wrapup::get_report,
        crate::api::wrapup::delete_wrapup_handler,
    ),
    components(schemas(
        api_types::wrapup::GenerateWrapUpRequest,
        api_types::wrapup::WrapUpGeneratedResponse,
        api_types::wrapup::WrapUpStatusResponse,
        api_types::wrapup::WrapUpListResponse,
    ))
)]
pub struct WrapUpDoc;
