use std::sync::Arc;

use domain::{
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    commands::{FollowCommand, RejectFollowCommand},
    deps::SocialCommandDeps,
    follow, reject,
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
async fn reject_follow_completes_without_error() {
    let (_social, _events, deps) = make_deps();
    let follower_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    follow::execute(
        &deps,
        FollowCommand {
            follower_id,
            target: SocialIdentity::Local(UserId::from_uuid(owner_id)),
        },
    )
    .await
    .unwrap();

    reject::execute(
        &deps,
        RejectFollowCommand {
            owner_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();
}
