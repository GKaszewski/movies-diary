use std::sync::Arc;

use activitypub_base::{ApUser, ApUserRepository};
use async_trait::async_trait;
use domain::{
    models::ProfileField,
    ports::{UserProfileFieldsRepository, UserRepository},
    value_objects::UserId,
};
use url::Url;

pub struct DomainUserRepoAdapter {
    pub repo: Arc<dyn UserRepository>,
    pub fields_repo: Arc<dyn UserProfileFieldsRepository>,
    pub base_url: String,
}

impl DomainUserRepoAdapter {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        fields_repo: Arc<dyn UserProfileFieldsRepository>,
        base_url: String,
    ) -> Self {
        Self { repo, fields_repo, base_url }
    }

    fn build_user(&self, u: &domain::models::User, fields: Vec<ProfileField>) -> ApUser {
        let avatar_url = u.avatar_path().and_then(|p| {
            Url::parse(&format!("{}/images/{}", self.base_url, p)).ok()
        });
        let banner_url = u.banner_path().and_then(|p| {
            Url::parse(&format!("{}/images/{}", self.base_url, p)).ok()
        });
        let profile_url = Url::parse(&format!("{}/u/{}", self.base_url, u.username().value())).ok();
        ApUser {
            id: u.id().value(),
            username: u.username().value().to_string(),
            bio: u.bio().map(|s| s.to_string()),
            avatar_url,
            banner_url,
            also_known_as: u.also_known_as().map(|s| s.to_string()),
            profile_url,
            attachment: fields,
        }
    }
}

#[async_trait]
impl ApUserRepository for DomainUserRepoAdapter {
    async fn find_by_id(&self, id: uuid::Uuid) -> anyhow::Result<Option<ApUser>> {
        let user_id = UserId::from_uuid(id);
        let user = match self.repo.find_by_id(&user_id).await? {
            Some(u) => u,
            None => return Ok(None),
        };
        let fields = self.fields_repo.get_fields(&user_id).await.unwrap_or_default();
        Ok(Some(self.build_user(&user, fields)))
    }

    async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<ApUser>> {
        use domain::value_objects::Username;
        let uname = Username::new(username.to_string()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let user = match self.repo.find_by_username(&uname).await? {
            Some(u) => u,
            None => return Ok(None),
        };
        let fields = self.fields_repo.get_fields(user.id()).await.unwrap_or_default();
        Ok(Some(self.build_user(&user, fields)))
    }

    async fn count_users(&self) -> anyhow::Result<usize> {
        Ok(self.repo.list_with_stats().await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .len())
    }
}
