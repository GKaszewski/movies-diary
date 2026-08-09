use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{FollowStatus, FollowTarget, SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    commands::SocialCmd,
    deps::{SocialCommandDeps, SocialQueryDeps},
    execute::execute_command,
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
async fn get_following_returns_accepted_targets() {
    let (social, _events, cmd_deps) = make_cmd_deps();
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
    };
    let follower = Uuid::new_v4();
    let target = UserId::from_uuid(Uuid::new_v4());

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id: follower,
            target: FollowTarget::Identity(SocialIdentity::Local(target.clone())),
        },
    )
    .await
    .unwrap();

    let before = crate::social::get_following::execute(&query_deps, follower)
        .await
        .unwrap();
    assert!(
        before.is_empty(),
        "a pending follow must not appear in get_following"
    );

    execute_command(
        &cmd_deps,
        SocialCmd::AcceptFollow {
            owner_id: target.value(),
            requester: SocialIdentity::Local(UserId::from_uuid(follower)),
        },
    )
    .await
    .unwrap();

    let actors = crate::social::get_following::execute(&query_deps, follower)
        .await
        .unwrap();

    assert_eq!(actors.len(), 1);
    assert_eq!(actors[0].identity, SocialIdentity::Local(target));
}

// ── Get followers ───────────────────────────────────────────────────────────

#[tokio::test]
async fn returns_accepted_followers() {
    let social = InMemorySocialRepository::new();
    let events = NoopEventPublisher::new();
    let cmd_deps = SocialCommandDeps {
        social_command: Arc::clone(&social) as _,
        event_publisher: Arc::clone(&events) as _,
    };
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
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

    let followers = crate::social::get_followers::execute(&query_deps, owner_id)
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
        event_publisher: Arc::clone(&events) as _,
    };
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
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

    let pending = crate::social::get_pending_followers::execute(&query_deps, owner_id)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
}

// ── get_relation ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_relation_reports_none_for_strangers() {
    use domain::ports::FollowGraphQuery;

    let (social, _events, _deps) = make_cmd_deps();
    let rel = FollowGraphQuery::get_relation(
        &*social,
        &UserId::from_uuid(Uuid::new_v4()),
        &SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4())),
    )
    .await
    .unwrap();

    assert_eq!(rel.following, None);
    assert_eq!(rel.followed_by, None);
}

#[tokio::test]
async fn get_relation_reports_pending_before_acceptance() {
    use domain::ports::FollowGraphQuery;

    let (social, _events, deps) = make_cmd_deps();
    let a = Uuid::new_v4();
    let b_id = UserId::from_uuid(Uuid::new_v4());

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id: a,
            target: FollowTarget::Identity(SocialIdentity::Local(b_id.clone())),
        },
    )
    .await
    .unwrap();

    let rel = FollowGraphQuery::get_relation(
        &*social,
        &UserId::from_uuid(a),
        &SocialIdentity::Local(b_id),
    )
    .await
    .unwrap();

    assert_eq!(rel.following, Some(FollowStatus::Pending));
    assert_eq!(rel.followed_by, None);
}

#[tokio::test]
async fn get_relation_reports_accepted_in_both_directions_after_mutual_follow() {
    use domain::ports::FollowGraphQuery;

    let (social, _events, deps) = make_cmd_deps();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let a_id = UserId::from_uuid(a);
    let b_id = UserId::from_uuid(b);

    for (from, to) in [(a, b_id.clone()), (b, a_id.clone())] {
        execute_command(
            &deps,
            SocialCmd::Follow {
                follower_id: from,
                target: FollowTarget::Identity(SocialIdentity::Local(to.clone())),
            },
        )
        .await
        .unwrap();
        let owner = if from == a { b } else { a };
        execute_command(
            &deps,
            SocialCmd::AcceptFollow {
                owner_id: owner,
                requester: SocialIdentity::Local(UserId::from_uuid(from)),
            },
        )
        .await
        .unwrap();
    }

    let rel = FollowGraphQuery::get_relation(&*social, &a_id, &SocialIdentity::Local(b_id))
        .await
        .unwrap();

    assert_eq!(rel.following, Some(FollowStatus::Accepted));
    assert_eq!(rel.followed_by, Some(FollowStatus::Accepted));
}

// ── get_pending_following ───────────────────────────────────────────────────

#[tokio::test]
async fn unaccepted_follow_appears_in_pending_following_not_in_following() {
    use domain::ports::FollowGraphQuery;

    let (social, _events, deps) = make_cmd_deps();
    let viewer = Uuid::new_v4();
    let target = UserId::from_uuid(Uuid::new_v4());

    execute_command(
        &deps,
        SocialCmd::Follow {
            follower_id: viewer,
            target: FollowTarget::Identity(SocialIdentity::Local(target.clone())),
        },
    )
    .await
    .unwrap();

    let pending = FollowGraphQuery::get_pending_following(&*social, &UserId::from_uuid(viewer))
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].identity, SocialIdentity::Local(target.clone()));

    let following = FollowGraphQuery::get_following(&*social, &UserId::from_uuid(viewer))
        .await
        .unwrap();
    assert!(
        following.is_empty(),
        "an unaccepted follow must not appear in get_following"
    );
}

// ── get_pending_following use case ──────────────────────────────────────────

#[tokio::test]
async fn get_pending_following_returns_unaccepted_targets_and_get_following_does_not() {
    let (social, _events, cmd_deps) = make_cmd_deps();
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
    };
    let follower = Uuid::new_v4();
    let target = UserId::from_uuid(Uuid::new_v4());

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id: follower,
            target: FollowTarget::Identity(SocialIdentity::Local(target.clone())),
        },
    )
    .await
    .unwrap();
    // deliberately NOT accepted

    let pending = crate::social::get_pending_following::execute(&query_deps, follower)
        .await
        .unwrap();
    let accepted = crate::social::get_following::execute(&query_deps, follower)
        .await
        .unwrap();

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].identity, SocialIdentity::Local(target));
    assert!(
        accepted.is_empty(),
        "a pending follow must not leak into get_following — the privacy invariant"
    );
}

// ── count_pending_followers use case ────────────────────────────────────────

#[tokio::test]
async fn count_pending_followers_drops_as_requests_are_accepted() {
    let (social, _events, cmd_deps) = make_cmd_deps();
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
    };
    let owner = UserId::from_uuid(Uuid::new_v4());
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    for follower in [a, b] {
        execute_command(
            &cmd_deps,
            SocialCmd::Follow {
                follower_id: follower,
                target: FollowTarget::Identity(SocialIdentity::Local(owner.clone())),
            },
        )
        .await
        .unwrap();
    }

    let before = crate::social::count_pending_followers::execute(&query_deps, owner.value())
        .await
        .unwrap();
    assert_eq!(before, 2);

    execute_command(
        &cmd_deps,
        SocialCmd::AcceptFollow {
            owner_id: owner.value(),
            requester: SocialIdentity::Local(UserId::from_uuid(a)),
        },
    )
    .await
    .unwrap();

    let after = crate::social::count_pending_followers::execute(&query_deps, owner.value())
        .await
        .unwrap();
    assert_eq!(
        after, 1,
        "accepting a request must remove it from the pending count"
    );
}

// ── get_relation use case ───────────────────────────────────────────────────

#[tokio::test]
async fn get_relation_use_case_reports_direction_asymmetrically() {
    let (social, _events, cmd_deps) = make_cmd_deps();
    let query_deps = SocialQueryDeps {
        follow_graph: Arc::clone(&social) as _,
        block_query: Arc::clone(&social) as _,
    };
    let viewer = Uuid::new_v4();
    let target = UserId::from_uuid(Uuid::new_v4());

    execute_command(
        &cmd_deps,
        SocialCmd::Follow {
            follower_id: viewer,
            target: FollowTarget::Identity(SocialIdentity::Local(target.clone())),
        },
    )
    .await
    .unwrap();

    let rel =
        crate::social::get_relation::execute(&query_deps, viewer, SocialIdentity::Local(target))
            .await
            .unwrap();

    assert_eq!(
        rel.following,
        Some(FollowStatus::Pending),
        "viewer -> target"
    );
    assert_eq!(rel.followed_by, None, "target has not followed back");
}
