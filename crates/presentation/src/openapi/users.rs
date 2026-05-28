use api_types::{
    ProfileFieldDto, ProfileResponse, UpdateProfileFieldsRequest, UserProfileResponse,
    UserStatsDto, UserSummaryDto, UsersResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::list_users,
        crate::handlers::api::get_user_profile,
        crate::handlers::api::get_profile,
        crate::handlers::api::update_profile_handler,
        crate::handlers::api::update_profile_fields_handler,
    ),
    components(schemas(
        UsersResponse,
        UserSummaryDto,
        UserProfileResponse,
        UserStatsDto,
        ProfileResponse,
        UpdateProfileFieldsRequest,
        ProfileFieldDto,
    ))
)]
pub struct UsersDoc;
