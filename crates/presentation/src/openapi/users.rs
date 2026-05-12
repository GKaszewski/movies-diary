use api_types::{ProfileResponse, UserProfileResponse, UserStatsDto, UserSummaryDto, UsersResponse};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::list_users,
        crate::handlers::api::get_user_profile,
        crate::handlers::api::get_profile,
        crate::handlers::api::update_profile_handler,
    ),
    components(schemas(
        UsersResponse,
        UserSummaryDto,
        UserProfileResponse,
        UserStatsDto,
        ProfileResponse,
    )),
)]
pub struct UsersDoc;
