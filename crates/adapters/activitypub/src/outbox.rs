use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::Path;
use serde::{Deserialize, Serialize};

use domain::value_objects::UserId;

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
        .map_err(|_| Error(anyhow::anyhow!("invalid user id")))?;
    let user_id = UserId::from_uuid(uuid);

    // verify user exists
    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(|e| Error(e.into()))?
        .ok_or_else(|| Error(anyhow::anyhow!("user not found")))?;

    let outbox_url = format!("{}/users/{}/outbox", data.base_url, user_id_str);

    Ok(FederationJson(OrderedCollection {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        kind: "OrderedCollection".to_string(),
        id: outbox_url,
        total_items: 0,
        ordered_items: vec![],
    }))
}
