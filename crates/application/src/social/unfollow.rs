use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::UnfollowCommand, deps::SocialCommandDeps};

pub async fn execute(deps: &SocialCommandDeps, cmd: UnfollowCommand) -> Result<(), DomainError> {
    let follower = UserId::from_uuid(cmd.follower_id);
    deps.social_command
        .unfollow(&follower, &cmd.target)
        .await?;
    deps.event_publisher
        .publish(&DomainEvent::Unfollowed {
            follower,
            target: cmd.target,
        })
        .await
}

#[cfg(test)]
#[path = "tests/unfollow.rs"]
mod tests;
