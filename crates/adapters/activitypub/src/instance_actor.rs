use std::sync::Arc;

use async_trait::async_trait;
use k_ap::{ApActorType, ApUser, ApUserRepository};

pub const INSTANCE_ACTOR_ID: uuid::Uuid =
    uuid::Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]);

pub struct InstanceActorUserRepo {
    inner: Arc<dyn ApUserRepository>,
    base_url: String,
}

impl InstanceActorUserRepo {
    pub fn new(inner: Arc<dyn ApUserRepository>, base_url: String) -> Self {
        Self { inner, base_url }
    }
}

fn instance_ap_user(base_url: &str) -> ApUser {
    ApUser {
        id: INSTANCE_ACTOR_ID,
        username: "instance".to_string(),
        display_name: None,
        bio: None,
        avatar_url: None,
        banner_url: None,
        also_known_as: vec![],
        profile_url: url::Url::parse(base_url).ok(),
        attachment: vec![],
        manually_approves_followers: false,
        discoverable: false,
        actor_type: ApActorType::Service,
        featured_url: None,
    }
}

#[async_trait]
impl ApUserRepository for InstanceActorUserRepo {
    async fn find_by_id(&self, id: uuid::Uuid) -> anyhow::Result<Option<ApUser>> {
        if id == INSTANCE_ACTOR_ID {
            return Ok(Some(instance_ap_user(&self.base_url)));
        }
        self.inner.find_by_id(id).await
    }

    async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<ApUser>> {
        if username == "instance" {
            return Ok(Some(instance_ap_user(&self.base_url)));
        }
        self.inner.find_by_username(username).await
    }

    async fn count_users(&self) -> anyhow::Result<usize> {
        self.inner.count_users().await
    }
}
