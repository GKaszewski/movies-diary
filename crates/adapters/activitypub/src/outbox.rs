use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::Path;
use serde::{Deserialize, Serialize};

use domain::{models::ReviewSource, value_objects::UserId};

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
    let user_id = UserId::from_uuid(uuid);

    data.user_repo
        .find_by_id(&user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let history = data
        .movie_repo
        .get_user_history(&user_id)
        .await
        .map_err(Error::from)?;

    let local_count = history
        .iter()
        .filter(|e| matches!(e.review().source(), ReviewSource::Local))
        .count();

    let outbox_url = format!("{}/users/{}/outbox", data.base_url, user_id_str);

    Ok(FederationJson(OrderedCollection {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        kind: "OrderedCollection".to_string(),
        id: outbox_url,
        total_items: local_count as u64,
        ordered_items: vec![],
    }))
}
