use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

use crate::dtos::{
    ActorListResponse, ActorUrlRequest, DiaryEntryDto, DiaryResponse, FollowRequest, LoginRequest,
    LoginResponse, LogReviewRequest, MovieDto, RegisterRequest, RemoteActorDto, ReviewDto,
    ReviewHistoryResponse,
};

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
        crate::handlers::api::get_following,
        crate::handlers::api::get_followers,
        crate::handlers::api::follow,
        crate::handlers::api::unfollow,
        crate::handlers::api::accept_follower,
        crate::handlers::api::reject_follower,
        crate::handlers::api::remove_follower,
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
    )),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;
