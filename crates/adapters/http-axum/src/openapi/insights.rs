use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(crate::api::insights::get_insights,),
    components(schemas(api_types::insights::InsightsResponse, api_types::insights::InsightDto,))
)]
pub struct InsightsDoc;
