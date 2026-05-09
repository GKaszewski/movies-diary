use domain::value_objects::ReviewId;
use url::Url;

/// Builds the canonical actor URL: `{base_url}/users/{user_id}`
pub fn actor_url(base_url: &str, user_id: uuid::Uuid) -> Url {
    Url::parse(&format!("{}/users/{}", base_url, user_id))
        .expect("base_url is always a valid URL prefix")
}

/// Builds the canonical review URL: `{base_url}/reviews/{review_id}`
pub fn review_url(base_url: &str, review_id: &ReviewId) -> Url {
    Url::parse(&format!("{}/reviews/{}", base_url, review_id.value()))
        .expect("base_url is always a valid URL prefix")
}
