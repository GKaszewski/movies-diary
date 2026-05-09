use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::Path;
use serde_json::json;

use domain::value_objects::UserId;

use crate::data::FederationData;
use crate::error::Error;
use crate::repository::FollowerStatus;

fn ordered_collection(id: String, total: usize, items: Vec<String>) -> serde_json::Value {
    json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "OrderedCollection",
        "id": id,
        "totalItems": total,
        "orderedItems": items,
    })
}

pub async fn followers_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&user_id_str)
            .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?,
    );

    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let followers = data
        .federation_repo
        .get_followers(user_id)
        .await
        .map_err(Error::from)?;

    let items: Vec<String> = followers
        .into_iter()
        .filter(|f| f.status == FollowerStatus::Accepted)
        .map(|f| f.actor.url)
        .collect();

    let id = format!("{}/users/{}/followers", data.base_url, user_id_str);
    Ok(FederationJson(ordered_collection(id, items.len(), items)))
}

pub async fn following_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&user_id_str)
            .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?,
    );

    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let following = data
        .federation_repo
        .get_following(user_id)
        .await
        .map_err(Error::from)?;

    let items: Vec<String> = following
        .into_iter()
        .map(|a| a.url)
        .collect();

    let id = format!("{}/users/{}/following", data.base_url, user_id_str);
    Ok(FederationJson(ordered_collection(id, items.len(), items)))
}
