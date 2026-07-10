use domain::{
    errors::DomainError,
    events::DomainEvent,
    value_objects::{SocialActor, UserId},
};

use super::{
    commands::SocialCmd,
    deps::{SocialCommandDeps, SocialQueryDeps},
    queries::SocialQry,
};

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

pub async fn execute_query(
    deps: &SocialQueryDeps,
    query: SocialQry,
) -> Result<Vec<SocialActor>, DomainError> {
    let user_id = match &query {
        SocialQry::GetFollowing { user_id }
        | SocialQry::GetFollowers { user_id }
        | SocialQry::GetPending { user_id }
        | SocialQry::GetBlocked { user_id } => UserId::from_uuid(*user_id),
    };
    match query {
        SocialQry::GetFollowing { .. } => deps.social_query.get_following(&user_id).await,
        SocialQry::GetFollowers { .. } => deps.social_query.get_followers(&user_id).await,
        SocialQry::GetPending { .. } => deps.social_query.get_pending_followers(&user_id).await,
        SocialQry::GetBlocked { .. } => deps.social_query.get_blocked(&user_id).await,
    }
}

#[cfg(test)]
#[path = "tests/execute.rs"]
mod tests;
