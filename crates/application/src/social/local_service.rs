use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{
        BlockQuery, FollowCommand, FollowGraphQuery, FollowQuery, FollowTargetResolver,
        FollowWrite, ResolvedFollow, SocialCommand, UserRepository,
    },
    value_objects::{
        FollowRelation, FollowStatus, FollowTarget, InstanceIdentity, SocialActor, SocialIdentity,
        UserId, Username,
    },
};

pub struct LocalSocialService {
    user_repo: Arc<dyn UserRepository>,
    follow_command: Arc<dyn FollowCommand>,
    follow_query: Arc<dyn FollowQuery>,
    instance: InstanceIdentity,
}

impl LocalSocialService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        follow_command: Arc<dyn FollowCommand>,
        follow_query: Arc<dyn FollowQuery>,
        instance: InstanceIdentity,
    ) -> Self {
        Self {
            user_repo,
            follow_command,
            follow_query,
            instance,
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
                let local_host = self.instance.host();
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

fn remote_unsupported(actor: &str) -> DomainError {
    DomainError::ValidationError(format!(
        "cannot reach remote actor {actor}: this instance was built without the federation feature"
    ))
}

#[async_trait]
impl SocialCommand for LocalSocialService {
    async fn follow(
        &self,
        follower: &UserId,
        target: &FollowTarget,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let identity = self.resolve_target_identity(target).await?;
        self.follow_resolved(follower, &identity, events).await
    }

    async fn unfollow(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(target);
        match target {
            SocialIdentity::Local(target_id) => {
                let follower_url = self.instance.actor_url_for(follower);
                self.follow_command
                    .apply(
                        &[
                            FollowWrite::RemoveFollow {
                                follower_id: follower.value(),
                                target_actor_url: actor_url,
                            },
                            FollowWrite::RemoveFollowerRecord {
                                local_user_id: target_id.value(),
                                follower_actor_url: follower_url,
                            },
                        ],
                        events,
                    )
                    .await
            }
            SocialIdentity::Remote { .. } => Err(remote_unsupported(&actor_url)),
        }
    }

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(requester);
        match requester {
            SocialIdentity::Local(requester_id) => {
                let owner_url = self.instance.actor_url_for(owner);
                self.follow_command
                    .apply(
                        &[
                            FollowWrite::UpdateFollowerStatus {
                                local_user_id: owner.value(),
                                follower_actor_url: actor_url,
                                status: FollowStatus::Accepted,
                            },
                            FollowWrite::UpdateFollowStatus {
                                follower_id: requester_id.value(),
                                target_actor_url: owner_url,
                                status: FollowStatus::Accepted,
                            },
                        ],
                        events,
                    )
                    .await
            }
            SocialIdentity::Remote { .. } => Err(remote_unsupported(&actor_url)),
        }
    }

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(requester);
        match requester {
            SocialIdentity::Local(requester_id) => {
                let owner_url = self.instance.actor_url_for(owner);
                self.follow_command
                    .apply(
                        &[
                            FollowWrite::UpdateFollowerStatus {
                                local_user_id: owner.value(),
                                follower_actor_url: actor_url,
                                status: FollowStatus::Rejected,
                            },
                            FollowWrite::RemoveFollow {
                                follower_id: requester_id.value(),
                                target_actor_url: owner_url,
                            },
                        ],
                        events,
                    )
                    .await
            }
            SocialIdentity::Remote { .. } => Err(remote_unsupported(&actor_url)),
        }
    }

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let actor_url = self.instance.actor_url_of(follower);
        match follower {
            SocialIdentity::Local(follower_id) => {
                let owner_url = self.instance.actor_url_for(owner);
                self.follow_command
                    .apply(
                        &[
                            FollowWrite::RemoveFollowerRecord {
                                local_user_id: owner.value(),
                                follower_actor_url: actor_url,
                            },
                            FollowWrite::RemoveFollow {
                                follower_id: follower_id.value(),
                                target_actor_url: owner_url,
                            },
                        ],
                        events,
                    )
                    .await
            }
            SocialIdentity::Remote { .. } => Err(remote_unsupported(&actor_url)),
        }
    }

    async fn block(
        &self,
        _blocker: &UserId,
        _target: &SocialIdentity,
        _events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(DomainError::ValidationError(
            "blocking requires the federation feature".into(),
        ))
    }

    async fn unblock(
        &self,
        _blocker: &UserId,
        _target: &SocialIdentity,
        _events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        Err(DomainError::ValidationError(
            "blocking requires the federation feature".into(),
        ))
    }
}

#[async_trait]
impl FollowGraphQuery for LocalSocialService {
    async fn get_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query.get_following(user.value()).await
    }

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query.get_followers(user.value()).await
    }

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query.get_pending_followers(user.value()).await
    }

    async fn get_pending_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        self.follow_query.get_pending_following(user.value()).await
    }

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError> {
        self.follow_query.count_following(user.value()).await
    }

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.follow_query.count_followers(user.value()).await
    }

    async fn count_pending_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        self.follow_query
            .count_pending_followers(user.value())
            .await
    }

    async fn get_relation(
        &self,
        viewer: &UserId,
        target: &SocialIdentity,
    ) -> Result<FollowRelation, DomainError> {
        let actor_url = self.instance.actor_url_of(target);
        self.follow_query
            .get_relation(viewer.value(), &actor_url)
            .await
    }
}

#[async_trait]
impl BlockQuery for LocalSocialService {
    async fn get_blocked(&self, _user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        Ok(vec![])
    }
}

#[async_trait]
impl FollowTargetResolver for LocalSocialService {
    async fn resolve_target(&self, target: &FollowTarget) -> Result<SocialIdentity, DomainError> {
        self.resolve_target_identity(target).await
    }
}

#[async_trait]
impl ResolvedFollow for LocalSocialService {
    async fn follow_resolved(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let SocialIdentity::Local(target_id) = target else {
            let actor_url = self.instance.actor_url_of(target);
            return Err(remote_unsupported(&actor_url));
        };
        if follower == target_id {
            return Err(DomainError::ValidationError(
                "Cannot follow yourself".into(),
            ));
        }
        let follower_url = self.instance.actor_url_for(follower);
        let target_url = self.instance.actor_url_for(target_id);
        self.follow_command
            .apply(
                &[
                    FollowWrite::AddFollower {
                        local_user_id: target_id.value(),
                        follower_actor_url: follower_url,
                        status: FollowStatus::Pending,
                    },
                    FollowWrite::AddFollow {
                        follower_id: follower.value(),
                        target_actor_url: target_url,
                        status: FollowStatus::Pending,
                    },
                ],
                events,
            )
            .await
    }
}
