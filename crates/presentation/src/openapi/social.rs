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
        crate::handlers::social::get_following,
        crate::handlers::social::get_followers,
        crate::handlers::social::get_pending_followers,
        crate::handlers::social::follow,
        crate::handlers::social::unfollow,
        crate::handlers::social::accept_follower,
        crate::handlers::social::reject_follower,
        crate::handlers::social::remove_follower,
        crate::handlers::social::get_blocked_domains_admin,
        crate::handlers::social::add_blocked_domain_admin,
        crate::handlers::social::remove_blocked_domain_admin,
        crate::handlers::social::block_actor_api,
        crate::handlers::social::unblock_actor_api,
        crate::handlers::social::get_blocked_actors_api,
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
