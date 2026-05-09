use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::Path;
use serde::{Deserialize, Serialize};

use crate::data::FederationData;
use crate::error::Error;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    #[serde(rename = "@context")]
    context: String,
    #[serde(rename = "type")]
    kind: String,
    id: String,
    total_items: u64,
    ordered_items: Vec<serde_json::Value>,
}

pub async fn outbox_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<OrderedCollection>, Error> {
    let uuid = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?;

    data.user_repo
        .find_by_id(uuid)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let objects = data
        .object_handler
        .get_local_objects_for_user(uuid)
        .await
        .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

    let outbox_url = format!("{}/users/{}/outbox", data.base_url, user_id_str);

    Ok(FederationJson(OrderedCollection {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        kind: "OrderedCollection".to_string(),
        id: outbox_url,
        total_items: objects.len() as u64,
        ordered_items: vec![],
    }))
}
