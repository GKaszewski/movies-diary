use async_trait::async_trait;
use uuid::Uuid;

use k_ap::{ActivityPubService, BlockedDomain, RemoteActor};

#[async_trait]
pub trait ActivityPubPort: Send + Sync {
    async fn actor_json(&self, user_id: &str) -> anyhow::Result<String>;
    async fn count_following(&self, local_user_id: Uuid) -> anyhow::Result<usize>;
    async fn count_accepted_followers(&self, local_user_id: Uuid) -> anyhow::Result<usize>;
    async fn get_pending_followers(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn follow(&self, local_user_id: Uuid, handle: &str) -> anyhow::Result<()>;
    async fn unfollow(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
    async fn accept_follower(
        &self,
        local_user_id: Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()>;
    async fn reject_follower(
        &self,
        local_user_id: Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()>;
    async fn get_following(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn get_accepted_followers(&self, local_user_id: Uuid)
    -> anyhow::Result<Vec<RemoteActor>>;
    async fn remove_follower(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
    async fn block_actor(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
    async fn unblock_actor(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
    async fn get_blocked_actors(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn add_blocked_domain(&self, domain: &str, reason: Option<&str>) -> anyhow::Result<()>;
    async fn remove_blocked_domain(&self, domain: &str) -> anyhow::Result<()>;
    async fn get_blocked_domains(&self) -> anyhow::Result<Vec<BlockedDomain>>;
    async fn import_remote_outbox(&self, outbox_url: &str, actor_url: &str) -> anyhow::Result<()>;
    async fn followers_collection_json(&self, user_id: Uuid, page: Option<u32>) -> anyhow::Result<String>;
    async fn following_collection_json(&self, user_id: Uuid, page: Option<u32>) -> anyhow::Result<String>;
}

#[async_trait]
impl ActivityPubPort for ActivityPubService {
    async fn actor_json(&self, user_id: &str) -> anyhow::Result<String> {
        self.actor_json(user_id).await
    }
    async fn count_following(&self, local_user_id: Uuid) -> anyhow::Result<usize> {
        self.count_following(local_user_id).await
    }
    async fn count_accepted_followers(&self, local_user_id: Uuid) -> anyhow::Result<usize> {
        self.count_accepted_followers(local_user_id).await
    }
    async fn get_pending_followers(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_pending_followers(local_user_id).await
    }
    async fn follow(&self, local_user_id: Uuid, handle: &str) -> anyhow::Result<()> {
        self.follow(local_user_id, handle).await
    }
    async fn unfollow(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()> {
        self.unfollow(local_user_id, actor_url).await
    }
    async fn accept_follower(
        &self,
        local_user_id: Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        self.accept_follower(local_user_id, remote_actor_url).await
    }
    async fn reject_follower(
        &self,
        local_user_id: Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        self.reject_follower(local_user_id, remote_actor_url).await
    }
    async fn get_following(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_following(local_user_id).await
    }
    async fn get_accepted_followers(
        &self,
        local_user_id: Uuid,
    ) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_accepted_followers(local_user_id).await
    }
    async fn remove_follower(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()> {
        self.remove_follower(local_user_id, actor_url).await
    }
    async fn block_actor(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()> {
        self.block_actor(local_user_id, actor_url).await
    }
    async fn unblock_actor(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()> {
        self.unblock_actor(local_user_id, actor_url).await
    }
    async fn get_blocked_actors(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_blocked_actors(local_user_id).await
    }
    async fn add_blocked_domain(&self, domain: &str, reason: Option<&str>) -> anyhow::Result<()> {
        self.add_blocked_domain(domain, reason).await
    }
    async fn remove_blocked_domain(&self, domain: &str) -> anyhow::Result<()> {
        self.remove_blocked_domain(domain).await
    }
    async fn get_blocked_domains(&self) -> anyhow::Result<Vec<BlockedDomain>> {
        self.get_blocked_domains().await
    }
    async fn import_remote_outbox(&self, outbox_url: &str, actor_url: &str) -> anyhow::Result<()> {
        self.import_remote_outbox(outbox_url, actor_url).await
    }
    async fn followers_collection_json(&self, user_id: Uuid, page: Option<u32>) -> anyhow::Result<String> {
        self.followers_collection_json(user_id, page).await
    }
    async fn following_collection_json(&self, user_id: Uuid, page: Option<u32>) -> anyhow::Result<String> {
        self.following_collection_json(user_id, page).await
    }
}

pub struct NoopActivityPubService;

#[async_trait]
impl ActivityPubPort for NoopActivityPubService {
    async fn actor_json(&self, _: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }
    async fn count_following(&self, _: Uuid) -> anyhow::Result<usize> {
        Ok(0)
    }
    async fn count_accepted_followers(&self, _: Uuid) -> anyhow::Result<usize> {
        Ok(0)
    }
    async fn get_pending_followers(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        Ok(vec![])
    }
    async fn follow(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn unfollow(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn accept_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn reject_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_following(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        Ok(vec![])
    }
    async fn get_accepted_followers(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        Ok(vec![])
    }
    async fn remove_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn block_actor(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn unblock_actor(&self, _: Uuid, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_blocked_actors(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        Ok(vec![])
    }
    async fn add_blocked_domain(&self, _: &str, _: Option<&str>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn remove_blocked_domain(&self, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_blocked_domains(&self) -> anyhow::Result<Vec<BlockedDomain>> {
        Ok(vec![])
    }
    async fn import_remote_outbox(&self, _: &str, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn followers_collection_json(&self, _: Uuid, _: Option<u32>) -> anyhow::Result<String> {
        Ok(String::new())
    }
    async fn following_collection_json(&self, _: Uuid, _: Option<u32>) -> anyhow::Result<String> {
        Ok(String::new())
    }
}
