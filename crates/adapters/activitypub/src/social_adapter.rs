use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{SocialCommand, SocialQuery, UserRepository},
    value_objects::{FollowTarget, SocialActor, SocialIdentity, UserId},
};

use k_ap::RemoteActor;

use super::ActivityPubPort;

pub struct CompositeSocialAdapter {
    ap_service: Arc<dyn ActivityPubPort>,
    user_repo: Arc<dyn UserRepository>,
    base_url: String,
}

impl CompositeSocialAdapter {
    pub fn new(
        ap_service: Arc<dyn ActivityPubPort>,
        user_repo: Arc<dyn UserRepository>,
        base_url: String,
    ) -> Self {
        Self {
            ap_service,
            user_repo,
            base_url,
        }
    }

    fn local_actor_url(&self, user_id: &UserId) -> String {
        format!("{}/users/{}", self.base_url, user_id.value())
    }

    fn actor_url_from_identity(&self, identity: &SocialIdentity) -> String {
        match identity {
            SocialIdentity::Local(uid) => self.local_actor_url(uid),
            SocialIdentity::Remote { actor_url } => actor_url.clone(),
        }
    }

    fn identity_from_actor_url(&self, url: &str) -> SocialIdentity {
        let prefix = format!("{}/users/", self.base_url);
        if let Some(uuid_str) = url.strip_prefix(&prefix)
            && let Ok(uuid) = uuid::Uuid::parse_str(uuid_str)
        {
            return SocialIdentity::Local(UserId::from_uuid(uuid));
        }
        SocialIdentity::Remote {
            actor_url: url.to_string(),
        }
    }

    fn remote_actor_to_social_actor(&self, actor: RemoteActor) -> SocialActor {
        let identity = self.identity_from_actor_url(&actor.url);
        SocialActor {
            identity,
            handle: actor.handle,
            display_name: actor.display_name,
            avatar_url: actor.avatar_url,
        }
    }

    async fn resolve_handle(&self, identity: &SocialIdentity) -> Result<String, DomainError> {
        match identity {
            SocialIdentity::Local(uid) => {
                let user = self
                    .user_repo
                    .find_by_id(uid)
                    .await?
                    .ok_or_else(|| DomainError::NotFound("User not found".into()))?;
                let host = url::Url::parse(&self.base_url)
                    .map(|u| u.host_str().unwrap_or("localhost").to_string())
                    .unwrap_or_else(|_| "localhost".to_string());
                Ok(format!("@{}@{}", user.username().value(), host))
            }
            SocialIdentity::Remote { actor_url } => Ok(actor_url.clone()),
        }
    }
}

fn ap_err(e: anyhow::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl SocialCommand for CompositeSocialAdapter {
    async fn follow(&self, follower: &UserId, target: &FollowTarget) -> Result<(), DomainError> {
        if let FollowTarget::Identity(SocialIdentity::Local(target_id)) = target
            && follower == target_id
        {
            return Err(DomainError::ValidationError(
                "Cannot follow yourself".into(),
            ));
        }
        let handle = match target {
            FollowTarget::Handle(h) => h.clone(),
            FollowTarget::Identity(id) => self.resolve_handle(id).await?,
        };
        self.ap_service
            .follow(follower.value(), &handle)
            .await
            .map_err(ap_err)
    }

    async fn unfollow(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(target);
        self.ap_service
            .unfollow(follower.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(requester);
        self.ap_service
            .accept_follower(owner.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(requester);
        self.ap_service
            .reject_follower(owner.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(follower);
        self.ap_service
            .remove_follower(owner.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn block(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(target);
        self.ap_service
            .block_actor(blocker.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn unblock(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(target);
        self.ap_service
            .unblock_actor(blocker.value(), &actor_url)
            .await
            .map_err(ap_err)
    }
}

#[async_trait]
impl SocialQuery for CompositeSocialAdapter {
    async fn get_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_following(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| self.remote_actor_to_social_actor(a))
            .collect())
    }

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_accepted_followers(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| self.remote_actor_to_social_actor(a))
            .collect())
    }

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_pending_followers(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| self.remote_actor_to_social_actor(a))
            .collect())
    }

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError> {
        self.ap_service
            .count_following(user.value())
            .await
            .map_err(ap_err)
    }

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.ap_service
            .count_accepted_followers(user.value())
            .await
            .map_err(ap_err)
    }

    async fn get_blocked(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_blocked_actors(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| self.remote_actor_to_social_actor(a))
            .collect())
    }

    async fn is_following(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<bool, DomainError> {
        let following = self.get_following(follower).await?;
        Ok(following.iter().any(|a| a.identity == *target))
    }

}
