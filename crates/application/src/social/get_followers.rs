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
    deps.follow_graph
        .get_followers(&UserId::from_uuid(user_id))
        .await
}
