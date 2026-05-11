use std::sync::Arc;

use activitypub_base::{ApUser, ApUserRepository};
use async_trait::async_trait;
use domain::{ports::UserRepository, value_objects::UserId};

pub struct DomainUserRepoAdapter(pub Arc<dyn UserRepository>);

#[async_trait]
impl ApUserRepository for DomainUserRepoAdapter {
    async fn find_by_id(&self, id: uuid::Uuid) -> anyhow::Result<Option<ApUser>> {
        let user_id = UserId::from_uuid(id);
        Ok(self.0.find_by_id(&user_id).await?.map(|u| ApUser {
            id: u.id().value(),
            username: u.username().value().to_string(),
            bio: u.bio().map(|s| s.to_string()),
            avatar_path: u.avatar_path().map(|s| s.to_string()),
        }))
    }

    async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<ApUser>> {
        use domain::value_objects::Username;
        let uname =
            Username::new(username.to_string()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(self.0.find_by_username(&uname).await?.map(|u| ApUser {
            id: u.id().value(),
            username: u.username().value().to_string(),
            bio: u.bio().map(|s| s.to_string()),
            avatar_path: u.avatar_path().map(|s| s.to_string()),
        }))
    }
}
