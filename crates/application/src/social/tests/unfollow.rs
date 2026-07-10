use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    commands::{FollowCommand, UnfollowCommand},
    deps::SocialCommandDeps,
    follow, unfollow,
};

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
async fn unfollow_emits_unfollowed_event() {
    let (_social, events, deps) = make_deps();
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

    unfollow::execute(
        &deps,
        UnfollowCommand {
            follower_id,
            target,
        },
    )
    .await
    .unwrap();

    let published = events.published();
    assert!(published
        .iter()
        .any(|e| matches!(e, DomainEvent::Unfollowed { .. })));
}
