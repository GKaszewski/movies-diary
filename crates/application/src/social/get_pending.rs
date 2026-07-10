use domain::{errors::DomainError, value_objects::{SocialIdentity, UserId}};

use super::{deps::SocialQueryDeps, queries::GetPendingFollowersQuery};

pub async fn execute(
    deps: &SocialQueryDeps,
    query: GetPendingFollowersQuery,
) -> Result<Vec<SocialIdentity>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.social_query.get_pending_followers(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_pending.rs"]
mod tests;
