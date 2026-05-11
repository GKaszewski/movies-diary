use std::sync::Arc;

use activitypub_base::{ApUser, ApUserRepository};
use async_trait::async_trait;
use domain::{ports::UserRepository, value_objects::UserId};
use url::Url;

pub struct DomainUserRepoAdapter {
    pub repo: Arc<dyn UserRepository>,
    pub base_url: String,
}

impl DomainUserRepoAdapter {
    pub fn new(repo: Arc<dyn UserRepository>, base_url: String) -> Self {
        Self { repo, base_url }
    }

    fn build_user(&self, u: &domain::models::User) -> ApUser {
        let avatar_url = u.avatar_path().and_then(|p| {
            Url::parse(&format!("{}/images/{}", self.base_url, p)).ok()
        });
        let profile_url = Url::parse(&format!("{}/u/{}", self.base_url, u.username().value())).ok();
        ApUser {
            id: u.id().value(),
            username: u.username().value().to_string(),
            bio: u.bio().map(|s| s.to_string()),
            avatar_url,
            profile_url,
        }
    }
}

#[async_trait]
impl ApUserRepository for DomainUserRepoAdapter {
    async fn find_by_id(&self, id: uuid::Uuid) -> anyhow::Result<Option<ApUser>> {
        let user_id = UserId::from_uuid(id);
        Ok(self.repo.find_by_id(&user_id).await?.as_ref().map(|u| self.build_user(u)))
    }

    async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<ApUser>> {
        use domain::value_objects::Username;
        let uname = Username::new(username.to_string()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(self.repo.find_by_username(&uname).await?.as_ref().map(|u| self.build_user(u)))
    }

    async fn count_users(&self) -> anyhow::Result<usize> {
        Ok(self.repo.list_with_stats().await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .len())
    }
}
