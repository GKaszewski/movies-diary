use std::sync::Arc;

use domain::{
    events::DomainEvent,
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    block,
    commands::{BlockCommand, UnblockCommand},
    deps::SocialCommandDeps,
    unblock,
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
async fn unblock_emits_actor_unblocked_event() {
    let (_social, events, deps) = make_deps();
    let target = SocialIdentity::Local(UserId::from_uuid(Uuid::new_v4()));
    let blocker_id = Uuid::new_v4();

    block::execute(
        &deps,
        BlockCommand {
            blocker_id,
            target: target.clone(),
        },
    )
    .await
    .unwrap();

    unblock::execute(&deps, UnblockCommand { blocker_id, target })
        .await
        .unwrap();

    let published = events.published();
    assert!(
        published
            .iter()
            .any(|e| matches!(e, DomainEvent::ActorUnblocked { .. }))
    );
}
