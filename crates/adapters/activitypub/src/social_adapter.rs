use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{FollowCommand, FollowQuery, SocialCommand, SocialQuery, UserRepository},
    value_objects::{FollowStatus, FollowTarget, SocialActor, SocialIdentity, UserId, Username},
};

use super::ActivityPubPort;

pub struct CompositeSocialAdapter {
    ap_service: Arc<dyn ActivityPubPort>,
    user_repo: Arc<dyn UserRepository>,
    follow_command: Arc<dyn FollowCommand>,
    follow_query: Arc<dyn FollowQuery>,
    base_url: String,
}

impl CompositeSocialAdapter {
    pub fn new(
        ap_service: Arc<dyn ActivityPubPort>,
        user_repo: Arc<dyn UserRepository>,
        follow_command: Arc<dyn FollowCommand>,
        follow_query: Arc<dyn FollowQuery>,
        base_url: String,
    ) -> Self {
        Self {
            ap_service,
            user_repo,
            follow_command,
            follow_query,
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

    async fn resolve_target_identity(
        &self,
        target: &FollowTarget,
    ) -> Result<SocialIdentity, DomainError> {
        match target {
            FollowTarget::Identity(id) => Ok(id.clone()),
            FollowTarget::Handle(handle) => {
                let host = handle.rsplit_once('@').map(|(_, h)| h).unwrap_or("");
                let local_host = SocialIdentity::host_from_base_url(&self.base_url);
                if host == local_host {
                    let username_str = handle
                        .trim_start_matches('@')
                        .split('@')
                        .next()
                        .unwrap_or("");
                    if let Ok(username) = Username::new(username_str.to_string())
                        && let Some(user) = self.user_repo.find_by_username(&username).await?
                    {
                        return Ok(SocialIdentity::Local(user.id().clone()));
                    }
                }
                Ok(SocialIdentity::Remote {
                    actor_url: handle.clone(),
                })
            }
        }
    }
}

fn ap_err(e: anyhow::Error) -> DomainError {
    DomainError::InfrastructureError(e.to_string())
}

#[async_trait]
impl SocialCommand for CompositeSocialAdapter {
    async fn follow(&self, follower: &UserId, target: &FollowTarget) -> Result<(), DomainError> {
        let identity = self.resolve_target_identity(target).await?;

        if let SocialIdentity::Local(ref target_id) = identity {
            if follower == target_id {
                return Err(DomainError::ValidationError(
                    "Cannot follow yourself".into(),
                ));
            }
            let follower_url = self.local_actor_url(follower);
            let target_url = self.local_actor_url(target_id);
            self.follow_command
                .add_follower(target_id.value(), &follower_url, FollowStatus::Pending)
                .await?;
            self.follow_command
                .add_follow(follower.value(), &target_url, FollowStatus::Pending)
                .await?;
            return Ok(());
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
                    SocialIdentity::format_local_handle(user.username().value(), &self.base_url)
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
        let actor_url = self.actor_url_from_identity(target);
        match target {
            SocialIdentity::Local(target_id) => {
                let follower_url = self.local_actor_url(follower);
                self.follow_command
                    .remove_follow(follower.value(), &actor_url)
                    .await?;
                self.follow_command
                    .remove_follower_record(target_id.value(), &follower_url)
                    .await?;
                Ok(())
            }
            SocialIdentity::Remote { .. } => self
                .ap_service
                .unfollow(follower.value(), &actor_url)
                .await
                .map_err(ap_err),
        }
    }

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(requester);
        match requester {
            SocialIdentity::Local(requester_id) => {
                let owner_url = self.local_actor_url(owner);
                self.follow_command
                    .update_follower_status(owner.value(), &actor_url, FollowStatus::Accepted)
                    .await?;
                self.follow_command
                    .update_follow_status(requester_id.value(), &owner_url, FollowStatus::Accepted)
                    .await?;
                Ok(())
            }
            SocialIdentity::Remote { .. } => self
                .ap_service
                .accept_follower(owner.value(), &actor_url)
                .await
                .map_err(ap_err),
        }
    }

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(requester);
        match requester {
            SocialIdentity::Local(requester_id) => {
                let owner_url = self.local_actor_url(owner);
                self.follow_command
                    .update_follower_status(owner.value(), &actor_url, FollowStatus::Rejected)
                    .await?;
                self.follow_command
                    .remove_follow(requester_id.value(), &owner_url)
                    .await?;
                Ok(())
            }
            SocialIdentity::Remote { .. } => self
                .ap_service
                .reject_follower(owner.value(), &actor_url)
                .await
                .map_err(ap_err),
        }
    }

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
    ) -> Result<(), DomainError> {
        let actor_url = self.actor_url_from_identity(follower);
        match follower {
            SocialIdentity::Local(follower_id) => {
                let owner_url = self.local_actor_url(owner);
                self.follow_command
                    .remove_follower_record(owner.value(), &actor_url)
                    .await?;
                self.follow_command
                    .remove_follow(follower_id.value(), &owner_url)
                    .await?;
                Ok(())
            }
            SocialIdentity::Remote { .. } => self
                .ap_service
                .remove_follower(owner.value(), &actor_url)
                .await
                .map_err(ap_err),
        }
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
        self.follow_query
            .get_following(user.value(), &self.base_url)
            .await
    }

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query
            .get_followers(user.value(), &self.base_url)
            .await
    }

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query
            .get_pending_followers(user.value(), &self.base_url)
            .await
    }

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError> {
        self.follow_query.count_following(user.value()).await
    }

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.follow_query.count_followers(user.value()).await
    }

    async fn get_blocked(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let actors = self
            .ap_service
            .get_blocked_actors(user.value())
            .await
            .map_err(ap_err)?;
        Ok(actors
            .into_iter()
            .map(|a| {
                let identity = SocialIdentity::from_actor_url(&a.url, &self.base_url);
                SocialActor {
                    identity,
                    handle: a.handle,
                    display_name: a.display_name,
                    avatar_url: a.avatar_url,
                }
            })
            .collect())
    }

    async fn is_following(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<bool, DomainError> {
        let actor_url = self.actor_url_from_identity(target);
        self.follow_query
            .is_following(follower.value(), &actor_url)
            .await
    }
}
