use super::*;
use crate::value_objects::{FollowTarget, SocialIdentity, UserId};

#[test]
fn follow_accepted_matches() {
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());
    let event = DomainEvent::FollowAccepted {
        owner: uid.clone(),
        requester: SocialIdentity::Remote {
            actor_url: "https://remote.example/users/alice".to_string(),
        },
    };
    let DomainEvent::FollowAccepted { requester, .. } = event else {
        panic!("wrong variant");
    };
    assert_eq!(
        requester,
        SocialIdentity::Remote {
            actor_url: "https://remote.example/users/alice".to_string()
        }
    );
}

#[test]
fn follow_requested_with_identity() {
    let follower = UserId::from_uuid(uuid::Uuid::new_v4());
    let target = UserId::from_uuid(uuid::Uuid::new_v4());
    let event = DomainEvent::FollowRequested {
        follower: follower.clone(),
        target: FollowTarget::Identity(SocialIdentity::Local(target.clone())),
    };
    assert!(matches!(
        event,
        DomainEvent::FollowRequested {
            target: FollowTarget::Identity(SocialIdentity::Local(_)),
            ..
        }
    ));
}

#[test]
fn follow_requested_with_handle() {
    let follower = UserId::from_uuid(uuid::Uuid::new_v4());
    let event = DomainEvent::FollowRequested {
        follower: follower.clone(),
        target: FollowTarget::Handle("@alice@remote.example".into()),
    };
    assert!(matches!(
        event,
        DomainEvent::FollowRequested {
            target: FollowTarget::Handle(_),
            ..
        }
    ));
}
