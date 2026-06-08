use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::wrapup::post_generate,
        crate::handlers::wrapup::get_list,
        crate::handlers::wrapup::get_status,
        crate::handlers::wrapup::get_report,
        crate::handlers::wrapup::delete_wrapup_handler,
    ),
    components(schemas(
        api_types::wrapup::GenerateWrapUpRequest,
        api_types::wrapup::WrapUpGeneratedResponse,
        api_types::wrapup::WrapUpStatusResponse,
        api_types::wrapup::WrapUpListResponse,
    ))
)]
pub struct WrapUpDoc;
