use api_types::{
    ProfileFieldDto, ProfileResponse, UpdateProfileFieldsRequest, UserProfileResponse,
    UserStatsDto, UserSummaryDto, UsersResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::users::list_users,
        crate::handlers::users::get_user_profile,
        crate::handlers::users::get_profile,
        crate::handlers::users::update_profile_handler,
        crate::handlers::users::update_profile_fields_handler,
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
