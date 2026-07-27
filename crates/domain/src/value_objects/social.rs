use super::UserId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialIdentity {
    Local(UserId),
    Remote { actor_url: String },
}

impl SocialIdentity {
    pub fn from_actor_url(actor_url: &str, base_url: &str) -> Self {
        let prefix = format!("{}/users/", base_url);
        if let Some(uuid_str) = actor_url.strip_prefix(&prefix)
            && let Ok(uuid) = uuid::Uuid::parse_str(uuid_str)
        {
            return Self::Local(UserId::from_uuid(uuid));
        }
        Self::Remote {
            actor_url: actor_url.to_string(),
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote { .. })
    }

    pub fn format_local_handle(username: &str, base_url: &str) -> String {
        let host = Self::host_from_base_url(base_url);
        format!("@{}@{}", username, host)
    }

    pub fn host_from_base_url(base_url: &str) -> &str {
        base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("localhost")
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
