use domain::events::DomainEvent;
use domain::value_objects::{FollowTarget, SocialIdentity, UserId};

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

#[test]
fn retry_backoff_doubles_from_ten_seconds_and_caps_at_five_minutes() {
    use domain::events::retry_backoff;
    use std::time::Duration;
    let secs: Vec<u64> = (1..=8).map(|n| retry_backoff(n).as_secs()).collect();
    assert_eq!(secs, [10, 20, 40, 80, 160, 300, 300, 300]);
    assert_eq!(retry_backoff(0), Duration::from_secs(5));
    assert_eq!(retry_backoff(u32::MAX), Duration::from_secs(300));
}
