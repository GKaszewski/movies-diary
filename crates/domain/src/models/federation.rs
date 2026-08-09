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

impl Default for FederationFlags {
    fn default() -> Self {
        Self {
            goals: true,
            reviews: true,
            watchlist: true,
        }
    }
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

/// A domain-blocklist entry as presented to admin surfaces. Mirrors what the
/// federation adapter can supply; `blocked_at` is a pre-formatted string
/// because that is what the underlying store returns and every consumer
/// renders it verbatim.
#[derive(Debug, Clone)]
pub struct BlockedDomainInfo {
    pub domain: String,
    pub reason: Option<String>,
    pub blocked_at: String,
}

/// A remote actor the local instance follows, reduced to the two fields any
/// consumer actually reads. Deliberately narrower than the federation
/// library's actor type — widening it is a decision, not an oversight.
#[derive(Debug, Clone)]
pub struct FollowedActorInfo {
    pub url: String,
    pub outbox_url: Option<String>,
}
