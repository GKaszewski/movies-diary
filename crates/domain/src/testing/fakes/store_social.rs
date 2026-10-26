use async_trait::async_trait;

use crate::{
    errors::DomainError,
    events::DomainEvent,
    ports::{BlockQuery, FollowGraphQuery, SocialCommand},
    value_objects::{FollowRelation, FollowStatus, SocialActor, SocialIdentity, UserId},
};

use super::InMemoryStore;
use super::store::FollowState;

fn identity_to_actor(identity: &SocialIdentity) -> SocialActor {
    let handle = match identity {
        SocialIdentity::Local(uid) => format!("user-{}", uid.value()),
        SocialIdentity::Remote { actor_url } => actor_url.clone(),
    };
    SocialActor {
        identity: identity.clone(),
        handle,
        display_name: None,
        avatar_url: None,
    }
}

#[async_trait]
impl SocialCommand for InMemoryStore {
    async fn follow(
        &self,
        follower: &UserId,
        target: &crate::value_objects::FollowTarget,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let identity = match target {
            crate::value_objects::FollowTarget::Identity(id) => id.clone(),
            crate::value_objects::FollowTarget::Handle(h) => SocialIdentity::Remote {
                actor_url: h.clone(),
            },
        };
        if let SocialIdentity::Local(target_id) = &identity
            && follower == target_id
        {
            return Err(DomainError::ValidationError(
                "Cannot follow yourself".into(),
            ));
        }
        let mut store = self.follows.write().unwrap();
        let already = store
            .iter()
            .any(|(f, t, _)| *f == follower.value() && *t == identity);
        if already {
            return Err(DomainError::ValidationError("Already following".into()));
        }
        store.push((follower.value(), identity, FollowState::Pending));
        drop(store);
        self.record_events(events);
        Ok(())
    }

    async fn unfollow(
        &self,
        follower: &UserId,
        target: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.follows.write().unwrap();
        let before = store.len();
        store.retain(|(f, t, _)| !(*f == follower.value() && t == target));
        if store.len() == before {
            return Err(DomainError::NotFound(
                "Follow relationship not found".into(),
            ));
        }
        drop(store);
        self.record_events(events);
        Ok(())
    }

    async fn accept_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.follows.write().unwrap();
        let target_identity = SocialIdentity::Local(owner.clone());
        for (f, t, state) in store.iter_mut() {
            let requester_matches = match requester {
                SocialIdentity::Local(uid) => *f == uid.value(),
                SocialIdentity::Remote { actor_url } => {
                    if let SocialIdentity::Remote {
                        actor_url: stored_url,
                    } = requester
                    {
                        stored_url == actor_url
                    } else {
                        false
                    }
                }
            };
            if requester_matches && *t == target_identity && *state == FollowState::Pending {
                *state = FollowState::Accepted;
                drop(store);
                self.record_events(events);
                return Ok(());
            }
        }
        Err(DomainError::NotFound(
            "Pending follow request not found".into(),
        ))
    }

    async fn reject_follow(
        &self,
        owner: &UserId,
        requester: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.follows.write().unwrap();
        let target_identity = SocialIdentity::Local(owner.clone());
        let before = store.len();
        store.retain(|(f, t, state)| {
            let requester_matches = match requester {
                SocialIdentity::Local(uid) => *f == uid.value(),
                SocialIdentity::Remote { .. } => {
                    // For remote, match by checking the stored requester identity
                    false // simplified: reject removes by follower uuid match
                }
            };
            !(requester_matches && *t == target_identity && *state == FollowState::Pending)
        });
        if store.len() == before {
            return Err(DomainError::NotFound(
                "Pending follow request not found".into(),
            ));
        }
        drop(store);
        self.record_events(events);
        Ok(())
    }

    async fn remove_follower(
        &self,
        owner: &UserId,
        follower: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.follows.write().unwrap();
        let target_identity = SocialIdentity::Local(owner.clone());
        let before = store.len();
        store.retain(|(f, t, _)| {
            let follower_matches = match follower {
                SocialIdentity::Local(uid) => *f == uid.value(),
                SocialIdentity::Remote { .. } => false,
            };
            !(follower_matches && *t == target_identity)
        });
        if store.len() == before {
            return Err(DomainError::NotFound("Follower not found".into()));
        }
        drop(store);
        self.record_events(events);
        Ok(())
    }

    async fn block(
        &self,
        blocker: &UserId,
        target: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.blocked.write().unwrap();
        store.push((blocker.value(), target.clone()));
        // Also remove any existing follow relationships
        let mut follows = self.follows.write().unwrap();
        follows.retain(|(f, t, _)| !(*f == blocker.value() && t == target));
        drop(follows);
        drop(store);
        self.record_events(events);
        Ok(())
    }

    async fn unblock(
        &self,
        blocker: &UserId,
        target: &SocialIdentity,
        events: &[DomainEvent],
    ) -> Result<(), DomainError> {
        let mut store = self.blocked.write().unwrap();
        store.retain(|(b, t)| !(*b == blocker.value() && t == target));
        drop(store);
        self.record_events(events);
        Ok(())
    }
}

#[async_trait]
impl FollowGraphQuery for InMemoryStore {
    async fn get_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let store = self.follows.read().unwrap();
        Ok(store
            .iter()
            .filter(|(f, _, state)| *f == user.value() && *state == FollowState::Accepted)
            .map(|(_, t, _)| identity_to_actor(t))
            .collect())
    }

    async fn get_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let store = self.follows.read().unwrap();
        let target = SocialIdentity::Local(user.clone());
        Ok(store
            .iter()
            .filter(|(_, t, state)| *t == target && *state == FollowState::Accepted)
            .map(|(f, _, _)| {
                let id = SocialIdentity::Local(UserId::from_uuid(*f));
                identity_to_actor(&id)
            })
            .collect())
    }

    async fn get_pending_followers(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let store = self.follows.read().unwrap();
        let target = SocialIdentity::Local(user.clone());
        Ok(store
            .iter()
            .filter(|(_, t, state)| *t == target && *state == FollowState::Pending)
            .map(|(f, _, _)| {
                let id = SocialIdentity::Local(UserId::from_uuid(*f));
                identity_to_actor(&id)
            })
            .collect())
    }

    async fn get_pending_following(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let store = self.follows.read().unwrap();
        Ok(store
            .iter()
            .filter(|(f, _, state)| *f == user.value() && *state == FollowState::Pending)
            .map(|(_, t, _)| identity_to_actor(t))
            .collect())
    }

    async fn count_following(&self, user: &UserId) -> Result<usize, DomainError> {
        let store = self.follows.read().unwrap();
        Ok(store
            .iter()
            .filter(|(f, _, state)| *f == user.value() && *state == FollowState::Accepted)
            .count())
    }

    async fn count_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        let store = self.follows.read().unwrap();
        let target = SocialIdentity::Local(user.clone());
        Ok(store
            .iter()
            .filter(|(_, t, state)| *t == target && *state == FollowState::Accepted)
            .count())
    }

    async fn count_pending_followers(&self, user: &UserId) -> Result<usize, DomainError> {
        let store = self.follows.read().unwrap();
        let target = SocialIdentity::Local(user.clone());
        Ok(store
            .iter()
            .filter(|(_, t, state)| *t == target && *state == FollowState::Pending)
            .count())
    }

    async fn get_relation(
        &self,
        viewer: &UserId,
        target: &SocialIdentity,
    ) -> Result<FollowRelation, DomainError> {
        let store = self.follows.read().unwrap();
        let viewer_identity = SocialIdentity::Local(viewer.clone());
        let state_to_status = |s: &FollowState| match s {
            FollowState::Pending => FollowStatus::Pending,
            FollowState::Accepted => FollowStatus::Accepted,
        };
        Ok(FollowRelation {
            following: store
                .iter()
                .find(|(f, t, _)| *f == viewer.value() && t == target)
                .map(|(_, _, s)| state_to_status(s)),
            followed_by: store
                .iter()
                .find(|(f, t, _)| {
                    SocialIdentity::Local(UserId::from_uuid(*f)) == *target && *t == viewer_identity
                })
                .map(|(_, _, s)| state_to_status(s)),
        })
    }
}

#[async_trait]
impl BlockQuery for InMemoryStore {
    async fn get_blocked(&self, user: &UserId) -> Result<Vec<SocialActor>, DomainError> {
        let store = self.blocked.read().unwrap();
        Ok(store
            .iter()
            .filter(|(b, _)| *b == user.value())
            .map(|(_, t)| identity_to_actor(t))
            .collect())
    }
}
