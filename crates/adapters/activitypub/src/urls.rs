use domain::value_objects::{InstanceIdentity, ReviewId, UserId};
use url::Url;

pub fn actor_url(instance: &InstanceIdentity, user_id: uuid::Uuid) -> Url {
    Url::parse(&instance.actor_url_for(&UserId::from_uuid(user_id)))
        .expect("base_url is always a valid URL prefix")
}

pub fn review_url(instance: &InstanceIdentity, review_id: &ReviewId) -> Url {
    Url::parse(&format!(
        "{}/reviews/{}",
        instance.base_url(),
        review_id.value()
    ))
    .expect("base_url is always a valid URL prefix")
}

pub fn goal_url(instance: &InstanceIdentity, user_id: uuid::Uuid, year: u16) -> Url {
    Url::parse(&format!(
        "{}/users/{}/goals/{}",
        instance.base_url(),
        user_id,
        year
    ))
    .expect("base_url is always a valid URL prefix")
}

pub fn watchlist_entry_url(
    instance: &InstanceIdentity,
    user_id: uuid::Uuid,
    movie_id: uuid::Uuid,
) -> Url {
    Url::parse(&format!(
        "{}/users/{}/watchlist/{}",
        instance.base_url(),
        user_id,
        movie_id
    ))
    .expect("base_url is always a valid URL prefix")
}
