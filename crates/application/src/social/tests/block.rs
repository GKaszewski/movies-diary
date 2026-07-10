use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{block, commands::BlockCommand, deps::SocialCommandDeps};

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
async fn block_emits_actor_blocked_event() {
    let (_social, events, deps) = make_deps();

    block::execute(
        &deps,
        BlockCommand {
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
