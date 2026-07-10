use domain::{errors::DomainError, value_objects::{SocialIdentity, UserId}};

use super::{deps::SocialQueryDeps, queries::GetFollowingQuery};

pub async fn execute(
    deps: &SocialQueryDeps,
    query: GetFollowingQuery,
) -> Result<Vec<SocialIdentity>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.social_query.get_following(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_following.rs"]
mod tests;
