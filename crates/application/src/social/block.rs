use domain::{errors::DomainError, events::DomainEvent, value_objects::UserId};

use super::{commands::BlockCommand, deps::SocialCommandDeps};

pub async fn execute(deps: &SocialCommandDeps, cmd: BlockCommand) -> Result<(), DomainError> {
    let blocker = UserId::from_uuid(cmd.blocker_id);
    deps.social_command.block(&blocker, &cmd.target).await?;
    deps.event_publisher
        .publish(&DomainEvent::ActorBlocked {
            blocker,
            target: cmd.target,
        })
        .await
}

#[cfg(test)]
#[path = "tests/block.rs"]
mod tests;
