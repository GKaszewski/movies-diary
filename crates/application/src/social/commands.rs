use domain::value_objects::SocialIdentity;
use uuid::Uuid;

pub struct FollowCommand {
    pub follower_id: Uuid,
    pub target: SocialIdentity,
}

pub struct UnfollowCommand {
    pub follower_id: Uuid,
    pub target: SocialIdentity,
}

pub struct AcceptFollowCommand {
    pub owner_id: Uuid,
    pub requester: SocialIdentity,
}

pub struct RejectFollowCommand {
    pub owner_id: Uuid,
    pub requester: SocialIdentity,
}

pub struct RemoveFollowerCommand {
    pub owner_id: Uuid,
    pub follower: SocialIdentity,
}

pub struct BlockCommand {
    pub blocker_id: Uuid,
    pub target: SocialIdentity,
}

pub struct UnblockCommand {
    pub blocker_id: Uuid,
    pub target: SocialIdentity,
}
