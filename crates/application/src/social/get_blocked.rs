use domain::{
    errors::DomainError,
    value_objects::{SocialActor, UserId},
};
use uuid::Uuid;

use super::deps::SocialQueryDeps;

pub async fn execute(
    deps: &SocialQueryDeps,
    user_id: Uuid,
) -> Result<Vec<SocialActor>, DomainError> {
    deps.block_query
        .get_blocked(&UserId::from_uuid(user_id))
        .await
}
