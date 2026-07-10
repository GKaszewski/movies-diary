use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{FollowTarget, SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    accept,
    commands::{AcceptFollowCommand, FollowCommand, RemoveFollowerCommand},
    deps::SocialCommandDeps,
    follow, remove_follower,
};

fn make_deps() -> (
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

#[tokio::test]
async fn remove_follower_emits_follower_removed_event() {
    let (_social, events, deps) = make_deps();
    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    follow::execute(
        &deps,
        FollowCommand {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(owner_id))),
        },
    )
    .await
    .unwrap();

    accept::execute(
        &deps,
        AcceptFollowCommand {
            owner_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    remove_follower::execute(
        &deps,
        RemoveFollowerCommand {
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
