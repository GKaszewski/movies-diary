use super::*;
use crate::test_support::local_actor;
use domain::value_objects::{InstanceIdentity, SocialIdentity, UserId};

#[test]
fn local_blocked_dto_url_round_trips_back_to_the_same_identity() {
    let instance = InstanceIdentity::new("https://md.example");
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());

    let dto = social_actor_to_blocked_dto(local_actor(&uid), &instance);

    assert!(!dto.url.starts_with("local:"), "url was {}", dto.url);
    assert_eq!(instance.identify(&dto.url), SocialIdentity::Local(uid));
}
