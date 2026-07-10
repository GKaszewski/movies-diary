use std::sync::Arc;

use domain::{
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    commands::FollowCommand,
    deps::{SocialCommandDeps, SocialQueryDeps},
    follow, get_pending,
    queries::GetPendingFollowersQuery,
};

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

    follow::execute(
        &cmd_deps,
        FollowCommand {
            follower_id,
            target: SocialIdentity::Local(UserId::from_uuid(owner_id)),
        },
    )
    .await
    .unwrap();

    let pending = get_pending::execute(&query_deps, GetPendingFollowersQuery { user_id: owner_id })
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
}
