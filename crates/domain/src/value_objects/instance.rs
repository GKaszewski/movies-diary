use super::{SocialIdentity, UserId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceIdentity {
    base_url: String,
}

impl InstanceIdentity {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn host(&self) -> &str {
        self.base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("localhost")
    }

    pub fn actor_url_for(&self, user: &UserId) -> String {
        format!("{}/users/{}", self.base_url, user.value())
    }

    pub fn handle_for(&self, username: &str) -> String {
        format!("@{}@{}", username, self.host())
    }

    pub fn image_url_for(&self, path: &str) -> String {
        format!("{}/images/{}", self.base_url, path)
    }

    pub fn actor_url_of(&self, identity: &SocialIdentity) -> String {
        match identity {
            SocialIdentity::Local(uid) => self.actor_url_for(uid),
            SocialIdentity::Remote { actor_url } => actor_url.clone(),
        }
    }

    pub fn identify(&self, actor_url: &str) -> SocialIdentity {
        let prefix = format!("{}/users/", self.base_url);
        if let Some(uuid_str) = actor_url.strip_prefix(&prefix)
            && let Ok(uuid) = uuid::Uuid::parse_str(uuid_str)
        {
            return SocialIdentity::Local(UserId::from_uuid(uuid));
        }
        SocialIdentity::Remote {
            actor_url: actor_url.to_string(),
        }
    }
}
