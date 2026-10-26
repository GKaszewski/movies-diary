use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FollowRequest {
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActorUrlRequest {
    pub actor_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoteActorDto {
    pub handle: String,
    pub display_name: Option<String>,
    pub url: String,
    pub user_id: Option<Uuid>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActorListResponse {
    pub actors: Vec<RemoteActorDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BlockedDomainResponse {
    pub domain: String,
    pub reason: Option<String>,
    pub blocked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddBlockedDomainRequest {
    pub domain: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BlockedActorResponse {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FollowStateDto {
    None,
    Pending,
    Accepted,
    Rejected,
}

impl From<Option<domain::value_objects::FollowStatus>> for FollowStateDto {
    fn from(s: Option<domain::value_objects::FollowStatus>) -> Self {
        use domain::value_objects::FollowStatus as F;
        match s {
            None => Self::None,
            Some(F::Pending) => Self::Pending,
            Some(F::Accepted) => Self::Accepted,
            Some(F::Rejected) => Self::Rejected,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FollowRelationResponse {
    pub following: FollowStateDto,
    pub followed_by: FollowStateDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PendingCountResponse {
    pub count: usize,
}
