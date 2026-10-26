#[cfg(feature = "federation")]
use api_types::{
    ActorListResponse, ActorUrlRequest, AddBlockedDomainRequest, BlockedActorResponse,
    BlockedDomainResponse, FollowRelationResponse, FollowRequest, PendingCountResponse,
    RemoteActorDto,
};
#[cfg(feature = "federation")]
use utoipa::OpenApi;

#[cfg(feature = "federation")]
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::social::get_following,
        crate::api::social::get_followers,
        crate::api::social::get_pending_followers,
        crate::api::social::get_pending_follower_count,
        crate::api::social::get_pending_following,
        crate::api::social::get_relationship,
        crate::api::social::follow,
        crate::api::social::unfollow,
        crate::api::social::accept_follower,
        crate::api::social::reject_follower,
        crate::api::social::remove_follower,
        crate::federation::blocklist_api::get_blocked_domains_admin,
        crate::federation::blocklist_api::add_blocked_domain_admin,
        crate::federation::blocklist_api::remove_blocked_domain_admin,
        crate::federation::blocklist_api::block_actor_api,
        crate::federation::blocklist_api::unblock_actor_api,
        crate::federation::blocklist_api::get_blocked_actors_api,
    ),
    components(schemas(
        ActorListResponse,
        RemoteActorDto,
        FollowRequest,
        ActorUrlRequest,
        BlockedDomainResponse,
        AddBlockedDomainRequest,
        BlockedActorResponse,
        FollowRelationResponse,
        PendingCountResponse,
    ))
)]
pub struct SocialDoc;
