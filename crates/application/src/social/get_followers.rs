use domain::{
    errors::DomainError,
    value_objects::{SocialActor, UserId},
};

use super::{deps::SocialQueryDeps, queries::GetFollowersQuery};

pub async fn execute(
    deps: &SocialQueryDeps,
    query: GetFollowersQuery,
) -> Result<Vec<SocialActor>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.social_query.get_followers(&user_id).await
}

#[cfg(test)]
#[path = "tests/get_followers.rs"]
mod tests;
