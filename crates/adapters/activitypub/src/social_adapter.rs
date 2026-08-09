use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{BlockQuery, FollowGraphQuery, LocalSocial, SocialCommand, UserRepository},
    value_objects::{
        FollowRelation, FollowTarget, InstanceIdentity, SocialActor, SocialIdentity, UserId,
    },
};

use k_ap::ActivityPubService;

pub struct CompositeSocialAdapter {
    local: Arc<dyn LocalSocial>,
    ap_service: Arc<ActivityPubService>,
    user_repo: Arc<dyn UserRepository>,
    instance: InstanceIdentity,
}

impl CompositeSocialAdapter {
    pub fn new(
        local: Arc<dyn LocalSocial>,
        ap_service: Arc<ActivityPubService>,
        user_repo: Arc<dyn UserRepository>,
        instance: InstanceIdentity,
    ) -> Self {
        Self {
            local,
            ap_service,
            user_repo,
            instance,
        }
    }
}

fn ap_err(e: anyhow::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl SocialCommand for CompositeSocialAdapter {
    async fn follow(&self, follower: &UserId, target: &FollowTarget) -> Result<(), DomainError> {
        let identity = self.local.resolve_target(target).await?;

        if let SocialIdentity::Local(_) = identity {
            return self.local.follow_resolved(follower, &identity).await;
        }

        let handle = match target {
            FollowTarget::Handle(h) => h.clone(),
            FollowTarget::Identity(id) => match id {
                SocialIdentity::Local(uid) => {
                    let user = self
                        .user_repo
                        .find_by_id(uid)
                        .await?
                        .ok_or_else(|| DomainError::NotFound("User not found".into()))?;
                    self.instance.handle_for(user.username().value())
                }
                SocialIdentity::Remote { actor_url } => actor_url.clone(),
            },
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
        match target {
            SocialIdentity::Local(_) => self.local.unfollow(follower, target).await,
            SocialIdentity::Remote { .. } => self
                .ap_service
                .unfollow(follower.value(), &self.instance.actor_url_of(target))
                .await
                .map_err(ap_err),
        }
    }

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        match requester {
            SocialIdentity::Local(_) => self.local.accept_follow(owner, requester).await,
            SocialIdentity::Remote { .. } => self
                .ap_service
                .accept_follower(owner.value(), &self.instance.actor_url_of(requester))
                .await
                .map_err(ap_err),
        }
    }

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        match requester {
            SocialIdentity::Local(_) => self.local.reject_follow(owner, requester).await,
            SocialIdentity::Remote { .. } => self
                .ap_service
                .reject_follower(owner.value(), &self.instance.actor_url_of(requester))
                .await
                .map_err(ap_err),
        }
    }

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
    ) -> Result<(), DomainError> {
        match follower {
            SocialIdentity::Local(_) => self.local.remove_follower(owner, follower).await,
            SocialIdentity::Remote { .. } => self
                .ap_service
                .remove_follower(owner.value(), &self.instance.actor_url_of(follower))
                .await
                .map_err(ap_err),
        }
    }

    async fn block(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(target);
        self.ap_service
            .block_actor(blocker.value(), &actor_url)
            .await
            .map_err(ap_err)
    }

    async fn unblock(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(target);
        self.ap_service
            .unblock_actor(blocker.value(), &actor_url)
            .await
            .map_err(ap_err)
    }
}

#[async_trait]
impl FollowGraphQuery for CompositeSocialAdapter {
    async fn get_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.local.get_following(user).await
    }

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.local.get_followers(user).await
    }

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.local.get_pending_followers(user).await
    }

    async fn get_pending_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.local.get_pending_following(user).await
    }

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError> {
        self.local.count_following(user).await
    }

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.local.count_followers(user).await
    }

    async fn count_pending_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.local.count_pending_followers(user).await
    }

    async fn get_relation(
        &self,
        viewer: &UserId,
        target: &SocialIdentity,
    ) -> Result<FollowRelation, DomainError> {
        self.local.get_relation(viewer, target).await
    }
}

#[async_trait]
impl BlockQuery for CompositeSocialAdapter {
    async fn get_blocked(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_blocked_actors(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| {
                let identity = self.instance.identify(&a.url);
                SocialActor {
                    identity,
                    handle: a.handle,
                    display_name: a.display_name,
                    avatar_url: a.avatar_url,
                }
            })
            .collect())
    }
}
