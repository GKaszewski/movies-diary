use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::UnblockCommand, deps::SocialCommandDeps};

pub async fn execute(deps: &SocialCommandDeps, cmd: UnblockCommand) -> Result<(), DomainError> {
    let blocker = UserId::from_uuid(cmd.blocker_id);
    deps.social_command.unblock(&blocker, &cmd.target).await?;
    deps.event_publisher
        .publish(&DomainEvent::ActorUnblocked {
            blocker,
            target: cmd.target,
        })
        .await
}

#[cfg(test)]
#[path = "tests/unblock.rs"]
mod tests;
