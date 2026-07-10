use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::FollowCommand, deps::SocialCommandDeps};

pub async fn execute(deps: &SocialCommandDeps, cmd: FollowCommand) -> Result<(), DomainError> {
    let follower = UserId::from_uuid(cmd.follower_id);
    deps.social_command.follow(&follower, &cmd.target).await?;
    deps.event_publisher
        .publish(&DomainEvent::FollowRequested {
            follower,
            target: cmd.target,
        })
        .await
}

#[cfg(test)]
#[path = "tests/follow.rs"]
mod tests;
