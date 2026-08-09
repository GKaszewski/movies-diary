use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::SocialCmd, deps::SocialCommandDeps};

pub async fn execute_command(deps: &SocialCommandDeps, cmd: SocialCmd) -> Result<(), DomainError> {
    let event = match cmd {
        SocialCmd::Follow {
            follower_id,
            target,
        } => {
            let follower = UserId::from_uuid(follower_id);
            deps.social_command.follow(&follower, &target).await?;
            DomainEvent::FollowRequested { follower, target }
        }
        SocialCmd::Unfollow {
            follower_id,
            target,
        } => {
            let follower = UserId::from_uuid(follower_id);
            deps.social_command.unfollow(&follower, &target).await?;
            DomainEvent::Unfollowed { follower, target }
        }
        SocialCmd::AcceptFollow {
            owner_id,
            requester,
        } => {
            let owner = UserId::from_uuid(owner_id);
            deps.social_command
                .accept_follow(&owner, &requester)
                .await?;
            DomainEvent::FollowAccepted { owner, requester }
        }
        SocialCmd::RejectFollow {
            owner_id,
            requester,
        } => {
            let owner = UserId::from_uuid(owner_id);
            deps.social_command
                .reject_follow(&owner, &requester)
                .await?;
            DomainEvent::FollowRejected { owner, requester }
        }
        SocialCmd::RemoveFollower { owner_id, follower } => {
            let owner = UserId::from_uuid(owner_id);
            deps.social_command
                .remove_follower(&owner, &follower)
                .await?;
            DomainEvent::FollowerRemoved { owner, follower }
        }
        SocialCmd::Block { blocker_id, target } => {
            let blocker = UserId::from_uuid(blocker_id);
            deps.social_command.block(&blocker, &target).await?;
            DomainEvent::ActorBlocked { blocker, target }
        }
        SocialCmd::Unblock { blocker_id, target } => {
            let blocker = UserId::from_uuid(blocker_id);
            deps.social_command.unblock(&blocker, &target).await?;
            DomainEvent::ActorUnblocked { blocker, target }
        }
    };
    deps.event_publisher.publish(&event).await
}

#[cfg(test)]
#[path = "tests/execute.rs"]
mod tests;
