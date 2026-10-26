use async_trait::async_trait;
use domain::errors::DomainError;
use domain::ports::{
    BlockQuery, FollowGraphQuery, FollowTargetResolver, LocalSocial, ResolvedFollow, SocialCommand,
};
use domain::value_objects::SocialIdentity;
use std::sync::Arc;

struct Stub;

#[async_trait]
impl SocialCommand for Stub {
    async fn follow(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::FollowTarget,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unfollow(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn accept_follow(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn reject_follow(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_follower(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn block(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unblock(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

#[async_trait]
impl FollowGraphQuery for Stub {
    async fn get_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn get_pending_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        Ok(vec![])
    }
    async fn count_following(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn count_pending_followers(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
    async fn get_relation(
        &self,
        _: &domain::value_objects::UserId,
        _: &domain::value_objects::SocialIdentity,
    ) -> Result<domain::value_objects::FollowRelation, DomainError> {
        Err(DomainError::NotFound("stub".into()))
    }
}

#[async_trait]
impl BlockQuery for Stub {
    async fn get_blocked(
        &self,
        _: &domain::value_objects::UserId,
    ) -> Result<Vec<domain::value_objects::SocialActor>, DomainError> {
        Ok(vec![])
    }
}

#[async_trait]
impl FollowTargetResolver for Stub {
    async fn resolve_target(
        &self,
        target: &domain::value_objects::FollowTarget,
    ) -> Result<SocialIdentity, DomainError> {
        match target {
            domain::value_objects::FollowTarget::Identity(id) => Ok(id.clone()),
            domain::value_objects::FollowTarget::Handle(handle) => Ok(SocialIdentity::Remote {
                actor_url: handle.clone(),
            }),
        }
    }
}

#[async_trait]
impl ResolvedFollow for Stub {
    async fn follow_resolved(
        &self,
        _: &domain::value_objects::UserId,
        _: &SocialIdentity,
        _: &[domain::events::DomainEvent],
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

#[test]
fn blanket_impl_yields_a_trait_object() {
    let local: Arc<dyn LocalSocial> = Arc::new(Stub);
    // Reachable through each supertrait without a separate Arc.
    let _: &dyn SocialCommand = local.as_ref();
    let _: &dyn FollowGraphQuery = local.as_ref();
    let _: &dyn BlockQuery = local.as_ref();
    let _: &dyn FollowTargetResolver = local.as_ref();
    let _: &dyn ResolvedFollow = local.as_ref();
}
