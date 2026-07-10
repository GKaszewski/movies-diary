use domain::value_objects::{FollowTarget, SocialIdentity};
use uuid::Uuid;

pub enum SocialCmd {
    Follow {
        follower_id: Uuid,
        target: FollowTarget,
    },
    Unfollow {
        follower_id: Uuid,
        target: SocialIdentity,
    },
    AcceptFollow {
        owner_id: Uuid,
        requester: SocialIdentity,
    },
    RejectFollow {
        owner_id: Uuid,
        requester: SocialIdentity,
    },
    RemoveFollower {
        owner_id: Uuid,
        follower: SocialIdentity,
    },
    Block {
        blocker_id: Uuid,
        target: SocialIdentity,
    },
    Unblock {
        blocker_id: Uuid,
        target: SocialIdentity,
    },
}
