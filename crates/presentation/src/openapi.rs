use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

use crate::dtos::{
    ActivityFeedResponse, DiaryEntryDto, DiaryResponse,
    DirectorStatDto, FeedEntryDto, LoginRequest, LoginResponse, LogReviewRequest,
    MonthActivityDto, MonthlyRatingDto, MovieDto, RegisterRequest, ReviewDto,
    ReviewHistoryResponse, UserProfileResponse, UserStatsDto, UserSummaryDto, UserTrendsDto,
    UsersResponse,
};
use crate::handlers::import::{
    ApiFieldMapping, ApplyMappingRequest, ConfirmRequest, SaveProfileRequest,
    SessionCreatedResponse, SessionStateResponse,
};
#[cfg(feature = "federation")]
use crate::dtos::{ActorListResponse, ActorUrlRequest, FollowRequest, RemoteActorDto};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[cfg(not(feature = "federation"))]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Movies Diary API",
        version = "1.0.0",
        description = "REST API for Movies Diary. Authenticate with `POST /api/v1/auth/login` to get a Bearer token."
    ),
    paths(
        crate::handlers::api::get_diary,
        crate::handlers::api::get_review_history,
        crate::handlers::api::post_review,
        crate::handlers::api::delete_review,
        crate::handlers::api::sync_poster,
        crate::handlers::api::login,
        crate::handlers::api::register,
        crate::handlers::api::export_diary,
        crate::handlers::api::get_activity_feed,
        crate::handlers::api::list_users,
        crate::handlers::api::get_user_profile,
        crate::handlers::import::api_post_session,
        crate::handlers::import::api_get_session,
        crate::handlers::import::api_put_mapping,
        crate::handlers::import::api_post_confirm,
        crate::handlers::import::api_get_profiles,
        crate::handlers::import::api_post_profile,
        crate::handlers::import::api_delete_profile,
    ),
    components(schemas(
        DiaryResponse,
        DiaryEntryDto,
        MovieDto,
        ReviewDto,
        LogReviewRequest,
        LoginRequest,
        LoginResponse,
        RegisterRequest,
        ReviewHistoryResponse,
        ActivityFeedResponse,
        FeedEntryDto,
        UsersResponse,
        UserSummaryDto,
        UserProfileResponse,
        UserStatsDto,
        MonthActivityDto,
        MonthlyRatingDto,
        DirectorStatDto,
        UserTrendsDto,
        SessionCreatedResponse,
        SessionStateResponse,
        ApiFieldMapping,
        ApplyMappingRequest,
        ConfirmRequest,
        SaveProfileRequest,
    )),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

#[cfg(feature = "federation")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Movies Diary API",
        version = "1.0.0",
        description = "REST API for Movies Diary. Authenticate with `POST /api/v1/auth/login` to get a Bearer token."
    ),
    paths(
        crate::handlers::api::get_diary,
        crate::handlers::api::get_review_history,
        crate::handlers::api::post_review,
        crate::handlers::api::delete_review,
        crate::handlers::api::sync_poster,
        crate::handlers::api::login,
        crate::handlers::api::register,
        crate::handlers::api::export_diary,
        crate::handlers::api::get_activity_feed,
        crate::handlers::api::list_users,
        crate::handlers::api::get_user_profile,
        crate::handlers::api::get_following,
        crate::handlers::api::get_followers,
        crate::handlers::api::get_pending_followers,
        crate::handlers::api::follow,
        crate::handlers::api::unfollow,
        crate::handlers::api::accept_follower,
        crate::handlers::api::reject_follower,
        crate::handlers::api::remove_follower,
        crate::handlers::import::api_post_session,
        crate::handlers::import::api_get_session,
        crate::handlers::import::api_put_mapping,
        crate::handlers::import::api_post_confirm,
        crate::handlers::import::api_get_profiles,
        crate::handlers::import::api_post_profile,
        crate::handlers::import::api_delete_profile,
    ),
    components(schemas(
        DiaryResponse,
        DiaryEntryDto,
        MovieDto,
        ReviewDto,
        LogReviewRequest,
        LoginRequest,
        LoginResponse,
        RegisterRequest,
        ReviewHistoryResponse,
        ActorListResponse,
        RemoteActorDto,
        FollowRequest,
        ActorUrlRequest,
        ActivityFeedResponse,
        FeedEntryDto,
        UsersResponse,
        UserSummaryDto,
        UserProfileResponse,
        UserStatsDto,
        MonthActivityDto,
        MonthlyRatingDto,
        DirectorStatDto,
        UserTrendsDto,
        SessionCreatedResponse,
        SessionStateResponse,
        ApiFieldMapping,
        ApplyMappingRequest,
        ConfirmRequest,
        SaveProfileRequest,
    )),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;
