use domain::value_objects::{SocialActor, SocialIdentity, UserId};

pub(crate) fn local_actor(uid: &UserId) -> SocialActor {
    SocialActor {
        identity: SocialIdentity::Local(uid.clone()),
        handle: "@gabriel@md.example".into(),
        display_name: None,
        avatar_url: None,
    }
}
