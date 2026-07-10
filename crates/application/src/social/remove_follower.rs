use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::RemoveFollowerCommand, deps::SocialCommandDeps};

pub async fn execute(
    deps: &SocialCommandDeps,
    cmd: RemoveFollowerCommand,
) -> Result<(), DomainError> {
    let owner = UserId::from_uuid(cmd.owner_id);
    deps.social_command
        .remove_follower(&owner, &cmd.follower)
        .await?;
    deps.event_publisher
        .publish(&DomainEvent::FollowerRemoved {
            owner,
            follower: cmd.follower,
        })
        .await
}

#[cfg(test)]
#[path = "tests/remove_follower.rs"]
mod tests;
