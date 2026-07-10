use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::AcceptFollowCommand, deps::SocialCommandDeps};

pub async fn execute(
    deps: &SocialCommandDeps,
    cmd: AcceptFollowCommand,
) -> Result<(), DomainError> {
    let owner = UserId::from_uuid(cmd.owner_id);
    deps.social_command
        .accept_follow(&owner, &cmd.requester)
        .await?;
    deps.event_publisher
        .publish(&DomainEvent::FollowAccepted {
            owner,
            requester: cmd.requester,
        })
        .await
}

#[cfg(test)]
#[path = "tests/accept.rs"]
mod tests;
