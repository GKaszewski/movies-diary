use domain::{errors::DomainError, value_objects::UserId};
use uuid::Uuid;

use super::deps::SocialQueryDeps;

pub async fn execute(deps: &SocialQueryDeps, user_id: Uuid) -> Result<usize, DomainError> {
    deps.follow_graph
        .count_pending_followers(&UserId::from_uuid(user_id))
        .await
}
