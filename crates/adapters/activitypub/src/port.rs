use async_trait::async_trait;
use uuid::Uuid;

use activitypub_base::{ActivityPubService, RemoteActor};

#[async_trait]
pub trait ActivityPubPort: Send + Sync {
    async fn actor_json(&self, user_id: &str) -> anyhow::Result<String>;
    async fn count_following(&self, local_user_id: Uuid) -> anyhow::Result<usize>;
    async fn count_accepted_followers(&self, local_user_id: Uuid) -> anyhow::Result<usize>;
    async fn get_pending_followers(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn follow(&self, local_user_id: Uuid, handle: &str) -> anyhow::Result<()>;
    async fn unfollow(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
    async fn accept_follower(&self, local_user_id: Uuid, remote_actor_url: &str) -> anyhow::Result<()>;
    async fn reject_follower(&self, local_user_id: Uuid, remote_actor_url: &str) -> anyhow::Result<()>;
    async fn get_following(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn get_accepted_followers(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>>;
    async fn remove_follower(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()>;
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
    async fn accept_follower(&self, local_user_id: Uuid, remote_actor_url: &str) -> anyhow::Result<()> {
        self.accept_follower(local_user_id, remote_actor_url).await
    }
    async fn reject_follower(&self, local_user_id: Uuid, remote_actor_url: &str) -> anyhow::Result<()> {
        self.reject_follower(local_user_id, remote_actor_url).await
    }
    async fn get_following(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_following(local_user_id).await
    }
    async fn get_accepted_followers(&self, local_user_id: Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        self.get_accepted_followers(local_user_id).await
    }
    async fn remove_follower(&self, local_user_id: Uuid, actor_url: &str) -> anyhow::Result<()> {
        self.remove_follower(local_user_id, actor_url).await
    }
}

pub struct NoopActivityPubService;

#[async_trait]
impl ActivityPubPort for NoopActivityPubService {
    async fn actor_json(&self, _: &str) -> anyhow::Result<String> { Ok(String::new()) }
    async fn count_following(&self, _: Uuid) -> anyhow::Result<usize> { Ok(0) }
    async fn count_accepted_followers(&self, _: Uuid) -> anyhow::Result<usize> { Ok(0) }
    async fn get_pending_followers(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> { Ok(vec![]) }
    async fn follow(&self, _: Uuid, _: &str) -> anyhow::Result<()> { Ok(()) }
    async fn unfollow(&self, _: Uuid, _: &str) -> anyhow::Result<()> { Ok(()) }
    async fn accept_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> { Ok(()) }
    async fn reject_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> { Ok(()) }
    async fn get_following(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> { Ok(vec![]) }
    async fn get_accepted_followers(&self, _: Uuid) -> anyhow::Result<Vec<RemoteActor>> { Ok(vec![]) }
    async fn remove_follower(&self, _: Uuid, _: &str) -> anyhow::Result<()> { Ok(()) }
}
