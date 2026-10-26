use api_types::{
    DeleteAccountRequest, ProfileFieldDto, ProfileResponse, ProfileViewData,
    UpdateProfileFieldsRequest, UserProfileBase, UserProfileResponse, UserStatsDto, UserSummaryDto,
    UsersResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::users::list_users,
        crate::api::user_profile::get_user_profile,
        crate::api::users::get_profile,
        crate::api::users::update_profile_handler,
        crate::api::users::update_profile_fields_handler,
        crate::api::users::delete_me,
        crate::api::users::delete_user,
    ),
    components(schemas(
        UsersResponse,
        UserSummaryDto,
        UserProfileBase,
        UserProfileResponse,
        ProfileViewData,
        UserStatsDto,
        ProfileResponse,
        UpdateProfileFieldsRequest,
        ProfileFieldDto,
        DeleteAccountRequest,
    ))
)]
pub struct UsersDoc;
