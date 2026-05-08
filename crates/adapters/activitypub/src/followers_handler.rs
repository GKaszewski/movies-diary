use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::Path;
use serde_json::json;

use domain::value_objects::UserId;

use crate::data::FederationData;
use crate::error::Error;

pub async fn followers_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&user_id_str)
            .map_err(|_| Error(anyhow::anyhow!("invalid user id")))?,
    );

    // verify user exists
    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(|e| Error(e.into()))?
        .ok_or_else(|| Error(anyhow::anyhow!("user not found")))?;

    let id = format!("{}/users/{}/followers", data.base_url, user_id_str);
    // TODO(ap): implement pagination
    Ok(FederationJson(json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "OrderedCollection",
        "id": id,
        "totalItems": 0,
        "orderedItems": []
    })))
}

pub async fn following_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = UserId::from_uuid(
        uuid::Uuid::parse_str(&user_id_str)
            .map_err(|_| Error(anyhow::anyhow!("invalid user id")))?,
    );

    // verify user exists
    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(|e| Error(e.into()))?
        .ok_or_else(|| Error(anyhow::anyhow!("user not found")))?;

    let id = format!("{}/users/{}/following", data.base_url, user_id_str);
    // TODO(ap): implement pagination
    Ok(FederationJson(json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "OrderedCollection",
        "id": id,
        "totalItems": 0,
        "orderedItems": []
    })))
}
