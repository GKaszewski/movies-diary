use super::*;
use crate::test_support::local_actor;
use domain::value_objects::{InstanceIdentity, SocialIdentity, UserId};

#[test]
fn local_template_url_round_trips_back_to_the_same_identity() {
    let instance = InstanceIdentity::new("https://md.example");
    let uid = UserId::from_uuid(uuid::Uuid::new_v4());

    let data = social_actor_to_template(local_actor(&uid), &instance);

    assert!(!data.url.starts_with("local:"), "url was {}", data.url);
    assert_eq!(instance.identify(&data.url), SocialIdentity::Local(uid));
}
