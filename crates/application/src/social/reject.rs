use domain::{errors::DomainError, value_objects::UserId};

use super::{commands::RejectFollowCommand, deps::SocialCommandDeps};

pub async fn execute(
    deps: &SocialCommandDeps,
    cmd: RejectFollowCommand,
) -> Result<(), DomainError> {
    let owner = UserId::from_uuid(cmd.owner_id);
    deps.social_command
        .reject_follow(&owner, &cmd.requester)
        .await
}

#[cfg(test)]
#[path = "tests/reject.rs"]
mod tests;
