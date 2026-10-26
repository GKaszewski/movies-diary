use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::SocialCmd, deps::SocialCommandDeps};

pub async fn execute_command(deps: &SocialCommandDeps, cmd: SocialCmd) -> Result<(), DomainError> {
    let command = &deps.social_command;
    match cmd {
        SocialCmd::Follow {
            follower_id,
            target,
        } => {
            let follower = UserId::from_uuid(follower_id);
            let events = [DomainEvent::FollowRequested {
                follower: follower.clone(),
                target: target.clone(),
            }];
            command.follow(&follower, &target, &events).await
        }
        SocialCmd::Unfollow {
            follower_id,
            target,
        } => {
            let follower = UserId::from_uuid(follower_id);
            let events = [DomainEvent::Unfollowed {
                follower: follower.clone(),
                target: target.clone(),
            }];
            command.unfollow(&follower, &target, &events).await
        }
        SocialCmd::AcceptFollow {
            owner_id,
            requester,
        } => {
            let owner = UserId::from_uuid(owner_id);
            let events = [DomainEvent::FollowAccepted {
                owner: owner.clone(),
                requester: requester.clone(),
            }];
            command.accept_follow(&owner, &requester, &events).await
        }
        SocialCmd::RejectFollow {
            owner_id,
            requester,
        } => {
            let owner = UserId::from_uuid(owner_id);
            let events = [DomainEvent::FollowRejected {
                owner: owner.clone(),
                requester: requester.clone(),
            }];
            command.reject_follow(&owner, &requester, &events).await
        }
        SocialCmd::RemoveFollower { owner_id, follower } => {
            let owner = UserId::from_uuid(owner_id);
            let events = [DomainEvent::FollowerRemoved {
                owner: owner.clone(),
                follower: follower.clone(),
            }];
            command.remove_follower(&owner, &follower, &events).await
        }
        SocialCmd::Block { blocker_id, target } => {
            let blocker = UserId::from_uuid(blocker_id);
            let events = [DomainEvent::ActorBlocked {
                blocker: blocker.clone(),
                target: target.clone(),
            }];
            command.block(&blocker, &target, &events).await
        }
        SocialCmd::Unblock { blocker_id, target } => {
            let blocker = UserId::from_uuid(blocker_id);
            let events = [DomainEvent::ActorUnblocked {
                blocker: blocker.clone(),
                target: target.clone(),
            }];
            command.unblock(&blocker, &target, &events).await
        }
    }
}
