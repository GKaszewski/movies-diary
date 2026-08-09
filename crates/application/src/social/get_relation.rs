use domain::{
    errors::DomainError,
    value_objects::{FollowRelation, SocialIdentity, UserId},
};
use uuid::Uuid;

use super::deps::SocialQueryDeps;

pub async fn execute(
    deps: &SocialQueryDeps,
    viewer_id: Uuid,
    target: SocialIdentity,
) -> Result<FollowRelation, DomainError> {
    deps.follow_graph
        .get_relation(&UserId::from_uuid(viewer_id), &target)
        .await
}
