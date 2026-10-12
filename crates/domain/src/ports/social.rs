use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::{
    errors::DomainError,
    models::{
        DiaryEntry, FederationFlags, RemoteActorInfo, RemoteGoalEntry, RemoteWatchlistEntry,
        WatchlistWithMovie,
    },
    value_objects::{FollowRelation, FollowTarget, MovieId, SocialActor, SocialIdentity, UserId},
};

// ── Unified social ports (ADR-0002) ─────────────────────────────────────────

#[async_trait]
pub trait SocialCommand: Send + Sync {
    async fn follow(&self, follower: &UserId, target: &FollowTarget) -> Result<(), DomainError>;

    async fn unfollow(&self, follower: &UserId, target: &SocialIdentity)
    -> Result<(), DomainError>;

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
    ) -> Result<(), DomainError>;

    async fn block(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError>;

    async fn unblock(&self, blocker: &UserId, target: &SocialIdentity) -> Result<(), DomainError>;
}

#[async_trait]
pub trait FollowGraphQuery: Send + Sync {
    async fn get_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError>;

    async fn get_pending_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError>;

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError>;

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError>;

    async fn count_pending_followers(&self, user: &UserId) -> Result<usize, DomainError>;

    async fn get_relation(
        &self,
        viewer: &UserId,
        target: &SocialIdentity,
    ) -> Result<FollowRelation, DomainError>;
}

#[async_trait]
pub trait BlockQuery: Send + Sync {
    async fn get_blocked(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError>;
}

#[async_trait]
pub trait FederationAdminQuery: Send + Sync {
    async fn list_all_followed_remote_actors(&self) -> Result<Vec<RemoteActorInfo>, DomainError>;
}

#[async_trait]
pub trait UserFederationSettingsQuery: Send + Sync {
    async fn get_federation_flags(&self, user_id: &UserId) -> Result<FederationFlags, DomainError>;
}

#[async_trait]
pub trait RemoteWatchlistRepository: Send + Sync {
    async fn save(&self, entry: RemoteWatchlistEntry) -> Result<(), DomainError>;
    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_actor_url(
        &self,
        actor_url: &str,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_derived_uuid(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Vec<RemoteWatchlistEntry>, DomainError>;
}

#[async_trait]
pub trait RemoteGoalRepository: Send + Sync {
    async fn save(&self, entry: RemoteGoalEntry) -> Result<(), DomainError>;
    async fn update_by_ap_id(
        &self,
        ap_id: &str,
        target: u32,
        current: u32,
    ) -> Result<(), DomainError>;
    async fn remove_by_ap_id(&self, ap_id: &str, actor_url: &str) -> Result<(), DomainError>;
    async fn remove_all_by_actor(&self, actor_url: &str) -> Result<(), DomainError>;
    async fn get_by_actor_url(&self, actor_url: &str) -> Result<Vec<RemoteGoalEntry>, DomainError>;
}

#[async_trait]
pub trait LocalApContentQuery: Send + Sync {
    async fn get_local_watchlist_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<WatchlistWithMovie>, DomainError>;
    async fn get_local_reviews_for_movie(
        &self,
        movie_id: &MovieId,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
    async fn get_local_reviews_page(
        &self,
        user_id: &UserId,
        before: Option<NaiveDateTime>,
        limit: usize,
    ) -> Result<Vec<DiaryEntry>, DomainError>;
}

#[async_trait]
pub trait FollowTargetResolver: Send + Sync {
    async fn resolve_target(&self, target: &FollowTarget) -> Result<SocialIdentity, DomainError>;
}

#[async_trait]
pub trait ResolvedFollow: Send + Sync {
    async fn follow_resolved(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
    ) -> Result<(), DomainError>;
}

pub trait LocalSocial:
    SocialCommand + FollowGraphQuery + BlockQuery + FollowTargetResolver + ResolvedFollow
{
}

impl<T: SocialCommand + FollowGraphQuery + BlockQuery + FollowTargetResolver + ResolvedFollow>
    LocalSocial for T
{
}

#[cfg(test)]
mod local_social_tests {
    use super::*;
    use std::sync::Arc;

    struct Stub;

    #[async_trait]
    impl SocialCommand for Stub {
        async fn follow(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::FollowTarget,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn unfollow(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn accept_follow(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn reject_follow(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn remove_follower(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn block(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn unblock(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[async_trait]
    impl FollowGraphQuery for Stub {
        async fn get_following(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<Vec<crate::value_objects::SocialActor>, DomainError> {
            Ok(vec![])
        }
        async fn get_followers(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<Vec<crate::value_objects::SocialActor>, DomainError> {
            Ok(vec![])
        }
        async fn get_pending_followers(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<Vec<crate::value_objects::SocialActor>, DomainError> {
            Ok(vec![])
        }
        async fn get_pending_following(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<Vec<crate::value_objects::SocialActor>, DomainError> {
            Ok(vec![])
        }
        async fn count_following(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn count_followers(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn count_pending_followers(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn get_relation(
            &self,
            _: &crate::value_objects::UserId,
            _: &crate::value_objects::SocialIdentity,
        ) -> Result<crate::value_objects::FollowRelation, DomainError> {
            Err(DomainError::NotFound("stub".into()))
        }
    }

    #[async_trait]
    impl BlockQuery for Stub {
        async fn get_blocked(
            &self,
            _: &crate::value_objects::UserId,
        ) -> Result<Vec<crate::value_objects::SocialActor>, DomainError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl FollowTargetResolver for Stub {
        async fn resolve_target(
            &self,
            target: &crate::value_objects::FollowTarget,
        ) -> Result<SocialIdentity, DomainError> {
            match target {
                crate::value_objects::FollowTarget::Identity(id) => Ok(id.clone()),
                crate::value_objects::FollowTarget::Handle(handle) => Ok(SocialIdentity::Remote {
                    actor_url: handle.clone(),
                }),
            }
        }
    }

    #[async_trait]
    impl ResolvedFollow for Stub {
        async fn follow_resolved(
            &self,
            _: &crate::value_objects::UserId,
            _: &SocialIdentity,
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
}
