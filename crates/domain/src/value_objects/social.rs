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
