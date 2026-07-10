use domain::{errors::DomainError, value_objects::{SocialActor, UserId}};

use super::{deps::SocialQueryDeps, queries::GetBlockedQuery};

pub async fn execute(
    deps: &SocialQueryDeps,
    query: GetBlockedQuery,
) -> Result<Vec<SocialActor>, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    deps.social_query.get_blocked(&user_id).await
}
