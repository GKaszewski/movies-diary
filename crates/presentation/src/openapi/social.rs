#[cfg(feature = "federation")]
use api_types::{
    ActorListResponse, ActorUrlRequest, AddBlockedDomainRequest, BlockedActorResponse,
    BlockedDomainResponse, FollowRequest, RemoteActorDto,
};
#[cfg(feature = "federation")]
use utoipa::OpenApi;

#[cfg(feature = "federation")]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::get_following,
        crate::handlers::api::get_followers,
        crate::handlers::api::get_pending_followers,
        crate::handlers::api::follow,
        crate::handlers::api::unfollow,
        crate::handlers::api::accept_follower,
        crate::handlers::api::reject_follower,
        crate::handlers::api::remove_follower,
        crate::handlers::api::get_blocked_domains_admin,
        crate::handlers::api::add_blocked_domain_admin,
        crate::handlers::api::remove_blocked_domain_admin,
        crate::handlers::api::block_actor_api,
        crate::handlers::api::unblock_actor_api,
        crate::handlers::api::get_blocked_actors_api,
    ),
    components(schemas(
        ActorListResponse,
        RemoteActorDto,
        FollowRequest,
        ActorUrlRequest,
        BlockedDomainResponse,
        AddBlockedDomainRequest,
        BlockedActorResponse,
    ))
)]
pub struct SocialDoc;
