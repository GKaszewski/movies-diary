use super::UserId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialIdentity {
    Local(UserId),
    Remote { actor_url: String },
}

impl SocialIdentity {
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FollowTarget {
    Identity(SocialIdentity),
    Handle(String),
}

#[derive(Clone, Debug)]
pub struct SocialActor {
    pub identity: SocialIdentity,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FollowRelation {
    pub following: Option<FollowStatus>,
    pub followed_by: Option<FollowStatus>,
}
