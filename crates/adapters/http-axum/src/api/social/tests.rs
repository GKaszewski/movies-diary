use super::*;
use crate::test_support::local_actor;
use domain::value_objects::{InstanceIdentity, SocialActor, SocialIdentity, UserId};

#[test]
fn local_actor_dto_url_round_trips_back_to_the_same_identity() {
    let instance = InstanceIdentity::new("https://md.example");
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());

    let dto = social_actor_to_dto(local_actor(&uid), &instance);

    assert!(!dto.url.starts_with("local:"), "url was {}", dto.url);
    assert_eq!(
        instance.identify(&dto.url),
        SocialIdentity::Local(uid),
        "a local actor's dto url must parse back as Local"
    );
}

#[test]
fn remote_actor_dto_url_is_passed_through_unchanged() {
    let instance = InstanceIdentity::new("https://md.example");
    let actor = SocialActor {
        identity: SocialIdentity::Remote {
            actor_url: "https://other.example/users/bob".into(),
        },
        handle: "@bob@other.example".into(),
        display_name: None,
        avatar_url: None,
    };

    let dto = social_actor_to_dto(actor, &instance);

    assert_eq!(dto.url, "https://other.example/users/bob");
}

#[test]
fn local_actor_dto_carries_user_id_and_avatar() {
    let instance = InstanceIdentity::new("https://md.example");
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());
    let mut actor = local_actor(&uid);
    actor.avatar_url = Some("https://md.example/images/a.webp".into());

    let dto = social_actor_to_dto(actor, &instance);

    assert_eq!(
        dto.user_id,
        Some(uid.value()),
        "local actors must carry user_id for internal links"
    );
    assert_eq!(
        dto.avatar_url.as_deref(),
        Some("https://md.example/images/a.webp")
    );
}

#[test]
fn remote_actor_dto_has_no_user_id() {
    let instance = InstanceIdentity::new("https://md.example");
    let actor = SocialActor {
        identity: SocialIdentity::Remote {
            actor_url: "https://other.example/users/bob".into(),
        },
        handle: "@bob@other.example".into(),
        display_name: None,
        avatar_url: None,
    };
    assert_eq!(social_actor_to_dto(actor, &instance).user_id, None);
}
