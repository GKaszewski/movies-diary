use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ApUser {
    pub id: uuid::Uuid,
    pub username: String,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
}

#[async_trait]
pub trait ApUserRepository: Send + Sync {
    async fn find_by_id(&self, id: uuid::Uuid) -> anyhow::Result<Option<ApUser>>;
    async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<ApUser>>;
}
