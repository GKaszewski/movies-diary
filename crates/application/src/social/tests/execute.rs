use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{FollowTarget, SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    commands::SocialCmd,
    deps::{SocialCommandDeps, SocialQueryDeps},
    execute::{execute_command, execute_query},
    queries::SocialQry,
};

fn make_cmd_deps() -> (
    Arc<InMemorySocialRepository>,
    Arc<NoopEventPublisher>,
    SocialCommandDeps,
) {
    let social = InMemorySocialRepository::new();
    let events = NoopEventPublisher::new();
    let deps = SocialCommandDeps {
        social_command: Arc::clone(&social) as _,
        social_query: Arc::clone(&social) as _,
        event_publisher: Arc::clone(&events) as _,
    };
    (social, events, deps)
}

// ── Follow ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn follow_emits_follow_requested_event() {
    let (_social, events, deps) = make_cmd_deps();

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id: Uuid::new_v4(),
            target: FollowTarget::Identity(SocialIdentity::Local(
                UserId::from_uuid(Uuid::new_v4()),
            )),
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::FollowRequested { .. }))
    );
}

#[tokio::test]
async fn cannot_follow_yourself() {
    let (_social, _events, deps) = make_cmd_deps();
    let user_id = Uuid::new_v4();

    let result = execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id: user_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(user_id))),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn cannot_follow_same_target_twice() {
    let (_social, _events, deps) = make_cmd_deps();
    let follower_id = Uuid::new_v4();
    let target = FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4())));

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target: target.clone(),
        },
    )
    .await
    .unwrap();

    let result = execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target,
        },
    )
    .await;

    assert!(result.is_err());
}

// ── Unfollow ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn unfollow_emits_unfollowed_event() {
    let (_social, events, deps) = make_cmd_deps();
    let follower_id = Uuid::new_v4();
    let target = SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4()));

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(target.clone()),
        },
    )
    .await
    .unwrap();

    execute_command(
        &deps,
        SocialCmd::Unfollow {
            follower_id,
            target,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::Unfollowed { .. }))
    );
}

// ── Accept ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn accept_follow_emits_follow_accepted_event() {
    let (_social, events, deps) = make_cmd_deps();
    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let requester = SocialIdentity::Local(UserId::from_uuid(follower_id));

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    execute_command(
        &deps,
        SocialCmd::AcceptFollow {
            owner_id,
            requester,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::FollowAccepted { .. }))
    );
}

// ── Reject ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn reject_follow_emits_follow_rejected_event() {
    let (_social, events, deps) = make_cmd_deps();
    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    execute_command(
        &deps,
        SocialCmd::RejectFollow {
            owner_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::FollowRejected { .. }))
    );
}

// ── Remove follower ─────────────────────────────────────────────────────────

#[tokio::test]
async fn remove_follower_emits_follower_removed_event() {
    let (_social, events, deps) = make_cmd_deps();
    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    execute_command(
        &deps,
        SocialCmd::AcceptFollow {
            owner_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    execute_command(
        &deps,
        SocialCmd::RemoveFollower {
            owner_id,
            follower: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::FollowerRemoved { .. }))
    );
}

// ── Block ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn block_emits_actor_blocked_event() {
    let (_social, events, deps) = make_cmd_deps();

    execute_command(
        &deps,
        SocialCmd::Block {
            blocker_id: Uuid::new_v4(),
            target: SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4())),
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::ActorBlocked { .. }))
    );
}

// ── Unblock ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn unblock_emits_actor_unblocked_event() {
    let (_social, events, deps) = make_cmd_deps();
    let target = SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4()));
    let blocker_id = Uuid::new_v4();

    execute_command(
        &deps,
        SocialCmd::Block {
            blocker_id,
            target: target.clone(),
        },
    )
    .await
    .unwrap();

    execute_command(&deps, SocialCmd::Unblock { blocker_id, target })
        .await
        .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::ActorUnblocked { .. }))
    );
}

// ── Get following ───────────────────────────────────────────────────────────

#[tokio::test]
async fn returns_accepted_follows() {
    let social = InMemorySocialRepository::new();
    let events = NoopEventPublisher::new();
    let cmd_deps = SocialCommandDeps {
        social_command: Arc::clone(&social) as _,
        social_query: Arc::clone(&social) as _,
        event_publisher: Arc::clone(&events) as _,
    };
    let query_deps = SocialQueryDeps {
        social_query: Arc::clone(&social) as _,
    };

    let follower_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(target_id))),
        },
    )
    .await
    .unwrap();

    // Pending follow should not appear
    let following = execute_query(
        &query_deps,
        SocialQry::GetFollowing {
            user_id: follower_id,
        },
    )
    .await
    .unwrap();
    assert!(following.is_empty());

    // Accept, then it should appear
    execute_command(
        &cmd_deps,
        SocialCmd::AcceptFollow {
            owner_id: target_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    let following = execute_query(
        &query_deps,
        SocialQry::GetFollowing {
            user_id: follower_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(following.len(), 1);
}

// ── Get followers ───────────────────────────────────────────────────────────

#[tokio::test]
async fn returns_accepted_followers() {
    let social = InMemorySocialRepository::new();
    let events = NoopEventPublisher::new();
    let cmd_deps = SocialCommandDeps {
        social_command: Arc::clone(&social) as _,
        social_query: Arc::clone(&social) as _,
        event_publisher: Arc::clone(&events) as _,
    };
    let query_deps = SocialQueryDeps {
        social_query: Arc::clone(&social) as _,
    };

    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    execute_command(
        &cmd_deps,
        SocialCmd::AcceptFollow {
            owner_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    let followers = execute_query(&query_deps, SocialQry::GetFollowers { user_id: owner_id })
        .await
        .unwrap();
    assert_eq!(followers.len(), 1);
}

// ── Get pending ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn returns_only_pending_followers() {
    let social = InMemorySocialRepository::new();
    let events = NoopEventPublisher::new();
    let cmd_deps = SocialCommandDeps {
        social_command: Arc::clone(&social) as _,
        social_query: Arc::clone(&social) as _,
        event_publisher: Arc::clone(&events) as _,
    };
    let query_deps = SocialQueryDeps {
        social_query: Arc::clone(&social) as _,
    };

    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    let pending = execute_query(&query_deps, SocialQry::GetPending { user_id: owner_id })
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
}
