#[derive(Debug, Clone)]
pub struct RemoteActorInfo {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingFollowerInfo {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct FederationFlags {
    pub goals: bool,
    pub reviews: bool,
    pub watchlist: bool,
}

#[derive(Debug, Clone)]
pub struct FederatedProfile {
    pub actor_url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
}
