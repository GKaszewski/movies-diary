use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    models::{User, UserRole},
    ports::{
        BlockQuery, FollowCommand, FollowGraphQuery, FollowQuery, SocialCommand, UserRepository,
    },
    testing::InMemoryUserRepository,
    value_objects::{
        Email, FollowRelation, FollowStatus, FollowTarget, InstanceIdentity, PasswordHash,
        SocialActor, SocialIdentity, UserId, Username,
    },
};
use uuid::Uuid;

use crate::social::local_service::LocalSocialService;

// ── Fakes ────────────────────────────────────────────────────────────────────
//
// No fake exists anywhere for FollowCommand/FollowQuery. `InMemorySocialRepository`
// (domain::testing) implements SocialCommand/FollowGraphQuery/BlockQuery — the same
// level as LocalSocialService itself — so it cannot stand in as its dependency.
// A single struct backs both ports, mirroring how one database table serves both
// sides of the follow edge in production.

struct FollowFakeStore {
    // (follower_id, target_actor_url, status) — written by add_follow/update_follow_status/remove_follow
    follows: Mutex<Vec<(Uuid, String, FollowStatus)>>,
    // (local_user_id, follower_actor_url, status) — written by add_follower/update_follower_status/remove_follower_record
    followers: Mutex<Vec<(Uuid, String, FollowStatus)>>,
}

impl FollowFakeStore {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            follows: Mutex::new(Vec::new()),
            followers: Mutex::new(Vec::new()),
        })
    }
}

fn fake_actor(url: &str) -> SocialActor {
    SocialActor {
        identity: SocialIdentity::Remote {
            actor_url: url.to_string(),
        },
        handle: url.to_string(),
        display_name: None,
        avatar_url: None,
    }
}

#[async_trait]
impl FollowCommand for FollowFakeStore {
    async fn add_follow(
        &self,
        follower_id: Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        self.follows
            .lock()
            .unwrap()
            .push((follower_id, target_actor_url.to_string(), status));
        Ok(())
    }

    async fn update_follow_status(
        &self,
        follower_id: Uuid,
        target_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let mut store = self.follows.lock().unwrap();
        if let Some(entry) = store
            .iter_mut()
            .find(|(f, t, _)| *f == follower_id && t == target_actor_url)
        {
            entry.2 = status;
        }
        Ok(())
    }

    async fn remove_follow(
        &self,
        follower_id: Uuid,
        target_actor_url: &str,
    ) -> Result<(), DomainError> {
        self.follows
            .lock()
            .unwrap()
            .retain(|(f, t, _)| !(*f == follower_id && t == target_actor_url));
        Ok(())
    }

    async fn add_follower(
        &self,
        local_user_id: Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        self.followers.lock().unwrap().push((
            local_user_id,
            follower_actor_url.to_string(),
            status,
        ));
        Ok(())
    }

    async fn update_follower_status(
        &self,
        local_user_id: Uuid,
        follower_actor_url: &str,
        status: FollowStatus,
    ) -> Result<(), DomainError> {
        let mut store = self.followers.lock().unwrap();
        if let Some(entry) = store
            .iter_mut()
            .find(|(u, f, _)| *u == local_user_id && f == follower_actor_url)
        {
            entry.2 = status;
        }
        Ok(())
    }

    async fn remove_follower_record(
        &self,
        local_user_id: Uuid,
        follower_actor_url: &str,
    ) -> Result<(), DomainError> {
        self.followers
            .lock()
            .unwrap()
            .retain(|(u, f, _)| !(*u == local_user_id && f == follower_actor_url));
        Ok(())
    }
}

#[async_trait]
impl FollowQuery for FollowFakeStore {
    async fn get_following(&self, user_id: Uuid) -> Result<Vec<SocialActor>, DomainError> {
        Ok(self
            .follows
            .lock()
            .unwrap()
            .iter()
            .filter(|(f, _, _)| *f == user_id)
            .map(|(_, t, _)| fake_actor(t))
            .collect())
    }

    async fn get_followers(&self, user_id: Uuid) -> Result<Vec<SocialActor>, DomainError> {
        Ok(self
            .followers
            .lock()
            .unwrap()
            .iter()
            .filter(|(u, _, _)| *u == user_id)
            .map(|(_, f, _)| fake_actor(f))
            .collect())
    }

    async fn get_pending_followers(&self, user_id: Uuid) -> Result<Vec<SocialActor>, DomainError> {
        Ok(self
            .followers
            .lock()
            .unwrap()
            .iter()
            .filter(|(u, _, s)| *u == user_id && *s == FollowStatus::Pending)
            .map(|(_, f, _)| fake_actor(f))
            .collect())
    }

    async fn get_pending_following(&self, user_id: Uuid) -> Result<Vec<SocialActor>, DomainError> {
        Ok(self
            .follows
            .lock()
            .unwrap()
            .iter()
            .filter(|(f, _, s)| *f == user_id && *s == FollowStatus::Pending)
            .map(|(_, t, _)| fake_actor(t))
            .collect())
    }

    async fn count_following(&self, user_id: Uuid) -> Result<usize, DomainError> {
        Ok(self
            .follows
            .lock()
            .unwrap()
            .iter()
            .filter(|(f, _, _)| *f == user_id)
            .count())
    }

    async fn count_followers(&self, user_id: Uuid) -> Result<usize, DomainError> {
        Ok(self
            .followers
            .lock()
            .unwrap()
            .iter()
            .filter(|(u, _, _)| *u == user_id)
            .count())
    }

    async fn count_pending_followers(&self, user_id: Uuid) -> Result<usize, DomainError> {
        Ok(self
            .followers
            .lock()
            .unwrap()
            .iter()
            .filter(|(u, _, s)| *u == user_id && *s == FollowStatus::Pending)
            .count())
    }

    async fn get_relation(
        &self,
        viewer_id: Uuid,
        target_actor_url: &str,
    ) -> Result<FollowRelation, DomainError> {
        let following = self
            .follows
            .lock()
            .unwrap()
            .iter()
            .find(|(f, t, _)| *f == viewer_id && t == target_actor_url)
            .map(|(_, _, s)| *s);
        let followed_by = self
            .followers
            .lock()
            .unwrap()
            .iter()
            .find(|(u, f, _)| *u == viewer_id && f == target_actor_url)
            .map(|(_, _, s)| *s);
        Ok(FollowRelation {
            following,
            followed_by,
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn instance() -> InstanceIdentity {
    InstanceIdentity::new("http://md.example")
}

async fn register_user(repo: &Arc<InMemoryUserRepository>, username: &str) -> UserId {
    let user = User::new(
        Email::new(format!("{username}@example.com")).unwrap(),
        Username::new(username.to_string()).unwrap(),
        PasswordHash::new("hashed-password".to_string()).unwrap(),
        UserRole::Standard,
    );
    let id = user.id().clone();
    repo.save(&user).await.unwrap();
    id
}

fn service(
    user_repo: Arc<InMemoryUserRepository>,
    follow_store: &Arc<FollowFakeStore>,
    instance: InstanceIdentity,
) -> LocalSocialService {
    LocalSocialService::new(
        user_repo as Arc<dyn UserRepository>,
        follow_store.clone() as Arc<dyn FollowCommand>,
        follow_store.clone() as Arc<dyn FollowQuery>,
        instance,
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn follow_local_user_writes_both_sides() {
    let user_repo = InMemoryUserRepository::new();
    let bob_id = register_user(&user_repo, "bob").await;
    let alice_id = register_user(&user_repo, "alice").await;
    let follow_store = FollowFakeStore::new();
    let instance = instance();
    let svc = service(user_repo, &follow_store, instance.clone());

    svc.follow(
        &alice_id,
        &FollowTarget::Handle("@bob@md.example".to_string()),
    )
    .await
    .unwrap();

    let bob_url = instance.actor_url_for(&bob_id);
    let alice_url = instance.actor_url_for(&alice_id);

    let followers = svc.get_followers(&bob_id).await.unwrap();
    assert!(
        followers.iter().any(|a| a.handle == alice_url),
        "add_follower should have recorded alice as bob's follower"
    );

    let following = svc.get_following(&alice_id).await.unwrap();
    assert!(
        following.iter().any(|a| a.handle == bob_url),
        "add_follow should have recorded bob in alice's following list"
    );
}

#[tokio::test]
async fn follow_self_is_rejected() {
    let user_repo = InMemoryUserRepository::new();
    let alice_id = register_user(&user_repo, "alice").await;
    let follow_store = FollowFakeStore::new();
    let svc = service(user_repo, &follow_store, instance());

    let result = svc
        .follow(
            &alice_id,
            &FollowTarget::Handle("@alice@md.example".to_string()),
        )
        .await;

    match result {
        Err(DomainError::ValidationError(msg)) => assert_eq!(
            msg, "Cannot follow yourself",
            "the self-follow guard must not collapse into the generic remote-target error"
        ),
        other => panic!("expected ValidationError, got {other:?}"),
    }
    assert_eq!(
        follow_store.follows.lock().unwrap().len(),
        0,
        "self-follow must write nothing to the follows side"
    );
    assert_eq!(
        follow_store.followers.lock().unwrap().len(),
        0,
        "self-follow must write nothing to the followers side"
    );
}

#[tokio::test]
async fn follow_remote_target_errors_rather_than_silently_succeeding() {
    let user_repo = InMemoryUserRepository::new();
    let alice_id = register_user(&user_repo, "alice").await;
    let follow_store = FollowFakeStore::new();
    let svc = service(user_repo, &follow_store, instance());

    let result = svc
        .follow(
            &alice_id,
            &FollowTarget::Handle("@carol@other.example".to_string()),
        )
        .await;

    match result {
        Err(DomainError::ValidationError(msg)) => {
            assert!(
                msg.contains("@carol@other.example"),
                "the remote-target error must name the specific handle that failed, \
                 not a generic message: {msg}"
            );
            assert!(
                msg != "Cannot follow yourself",
                "the remote-target error must not collapse into the self-follow message"
            );
        }
        other => panic!("expected ValidationError, got {other:?}"),
    }
    assert_eq!(
        follow_store.follows.lock().unwrap().len(),
        0,
        "a remote target must write nothing — this replaces NoopSocialCommand's silent Ok(())"
    );
    assert_eq!(follow_store.followers.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn reject_follow_updates_follower_status_but_removes_the_follow_row() {
    // Asymmetry with accept_follow, which updates the follow row's status instead
    // of removing it: reject_follow marks the follower row Rejected but deletes
    // the follow row outright. A test that only checked the follower row would
    // pass against a reject_follow "corrected" into accept_follow's symmetry —
    // the second assertion is the one that carries the weight.
    let user_repo = InMemoryUserRepository::new();
    let owner_id = register_user(&user_repo, "owner").await;
    let requester_id = register_user(&user_repo, "requester").await;
    let follow_store = FollowFakeStore::new();
    let instance = instance();
    let svc = service(user_repo, &follow_store, instance.clone());

    svc.follow(
        &requester_id,
        &FollowTarget::Handle("@owner@md.example".to_string()),
    )
    .await
    .unwrap();

    svc.reject_follow(&owner_id, &SocialIdentity::Local(requester_id.clone()))
        .await
        .unwrap();

    let requester_url = instance.actor_url_for(&requester_id);
    let owner_url = instance.actor_url_for(&owner_id);

    {
        let followers = follow_store.followers.lock().unwrap();
        let follower_row = followers
            .iter()
            .find(|(u, f, _)| *u == owner_id.value() && f == &requester_url)
            .expect("the follower row must still exist after rejection");
        assert_eq!(
            follower_row.2,
            FollowStatus::Rejected,
            "reject_follow must mark the follower row Rejected"
        );
    }

    let follows = follow_store.follows.lock().unwrap();
    assert!(
        !follows
            .iter()
            .any(|(f, t, _)| *f == requester_id.value() && t == &owner_url),
        "reject_follow must remove the follow row outright, not merely mark it Rejected — \
         this is the deliberate asymmetry with accept_follow"
    );
}

#[tokio::test]
async fn block_errors_and_get_blocked_is_empty() {
    let user_repo = InMemoryUserRepository::new();
    let alice_id = register_user(&user_repo, "alice").await;
    let follow_store = FollowFakeStore::new();
    let svc = service(user_repo, &follow_store, instance());

    let target = SocialIdentity::Remote {
        actor_url: "https://other.example/users/carol".to_string(),
    };

    let result = svc.block(&alice_id, &target).await;
    assert!(matches!(result, Err(DomainError::ValidationError(_))));

    let blocked = svc.get_blocked(&alice_id).await.unwrap();
    assert_eq!(blocked.len(), 0);
}
