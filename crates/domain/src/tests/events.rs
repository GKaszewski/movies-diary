use super::*;
use crate::value_objects::UserId;

#[test]
fn follow_accepted_matches() {
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());
    let event = DomainEvent::FollowAccepted {
        local_user_id: uid.clone(),
        remote_actor_url: "https://remote.example/users/alice".to_string(),
        outbox_url: "https://remote.example/users/alice/outbox".to_string(),
    };
    let DomainEvent::FollowAccepted { outbox_url, .. } = event else {
        panic!("wrong variant");
    };
    assert_eq!(outbox_url, "https://remote.example/users/alice/outbox");
}
