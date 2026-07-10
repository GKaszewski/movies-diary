use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{commands::FollowCommand, deps::SocialCommandDeps, follow};

fn make_deps() -> (Arc<InMemorySocialRepository>, Arc<NoopEventPublisher>, SocialCommandDeps) {
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
async fn follow_emits_follow_requested_event() {
    let (_social, events, deps) = make_deps();

    follow::execute(
        &deps,
        FollowCommand {
            follower_id: Uuid::new_v4(),
            target: SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4())),
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(published
        .iter()
        .any(|e| matches!(e, DomainEvent::FollowRequested { .. })));
}

#[tokio::test]
async fn cannot_follow_yourself() {
    let (_social, _events, deps) = make_deps();
    let user_id = Uuid::new_v4();

    let result = follow::execute(
        &deps,
        FollowCommand {
            follower_id: user_id,
            target: SocialIdentity::Local(UserId::from_uuid(user_id)),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn cannot_follow_same_target_twice() {
    let (_social, _events, deps) = make_deps();
    let follower_id = Uuid::new_v4();
    let target = SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4()));

    follow::execute(
        &deps,
        FollowCommand {
            follower_id,
            target: target.clone(),
        },
    )
    .await
    .unwrap();

    let result = follow::execute(
        &deps,
        FollowCommand {
            follower_id,
            target,
        },
    )
    .await;

    assert!(result.is_err());
}
