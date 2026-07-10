use std::sync::Arc;

use domain::{
    testing::{InMemorySocialRepository, NoopEventPublisher},
    value_objects::{FollowTarget, SocialIdentity, UserId},
};
use uuid::Uuid;

use crate::social::{
    accept,
    commands::{AcceptFollowCommand, FollowCommand},
    deps::{SocialCommandDeps, SocialQueryDeps},
    follow, get_following,
    queries::GetFollowingQuery,
};

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

    follow::execute(
        &cmd_deps,
        FollowCommand {
            follower_id,
            target: FollowTarget::Identity(SocialIdentity::Local(UserId::from_uuid(target_id))),
        },
    )
    .await
    .unwrap();

    // Pending follow should not appear
    let following = get_following::execute(
        &query_deps,
        GetFollowingQuery {
            user_id: follower_id,
        },
    )
    .await
    .unwrap();
    assert!(following.is_empty());

    // Accept, then it should appear
    accept::execute(
        &cmd_deps,
        AcceptFollowCommand {
            owner_id: target_id,
            requester: SocialIdentity::Local(UserId::from_uuid(follower_id)),
        },
    )
    .await
    .unwrap();

    let following = get_following::execute(
        &query_deps,
        GetFollowingQuery {
            user_id: follower_id,
        },
    )
    .await
    .unwrap();
    assert_eq!(following.len(), 1);
}
