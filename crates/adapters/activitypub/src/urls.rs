use url::Url;
use uuid::Uuid;
use domain::value_objects::{UserId, ReviewId};
use crate::error::Error;

/// Extracts a UserId from a URL like `https://example.com/users/{uuid}[/...]`
pub fn extract_user_id_from_url(url: &Url) -> Option<UserId> {
    let path = url.path();
    path.strip_prefix("/users/")
        .and_then(|s| s.split('/').next())
        .and_then(|uid_str| Uuid::parse_str(uid_str).ok())
        .map(UserId::from_uuid)
}

/// Generates a fresh activity URL: `{base_url}/activities/{uuid}`
pub fn activity_url(base_url: &str) -> Result<Url, Error> {
    Url::parse(&format!("{}/activities/{}", base_url, Uuid::new_v4()))
        .map_err(|e| Error::bad_request(anyhow::anyhow!(e)))
}

/// Builds the canonical actor URL: `{base_url}/users/{user_id}`
pub fn actor_url(base_url: &str, user_id: &UserId) -> Url {
    Url::parse(&format!("{}/users/{}", base_url, user_id.value()))
        .expect("base_url is always a valid URL prefix")
}

/// Builds the canonical review URL: `{base_url}/reviews/{review_id}`
pub fn review_url(base_url: &str, review_id: &ReviewId) -> Url {
    Url::parse(&format!("{}/reviews/{}", base_url, review_id.value()))
        .expect("base_url is always a valid URL prefix")
}

/// Stable Create-activity URL derived from review ID.
/// Deterministic so repeated backfills to different followers don't create duplicate posts.
pub fn create_activity_url(base_url: &str, review_id: &ReviewId) -> Result<Url, Error> {
    Url::parse(&format!("{}/activities/create/{}", base_url, review_id.value()))
        .map_err(|e| Error::bad_request(anyhow::anyhow!(e)))
}
