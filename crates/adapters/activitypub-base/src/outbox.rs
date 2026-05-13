use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use activitypub_federation::{
    config::Data, fetch::object_id::ObjectId, kinds::activity::CreateType,
    protocol::context::WithContext,
};

use crate::{activities::CreateActivity, data::FederationData, error::Error};

const PAGE_SIZE: usize = 20;

#[derive(Deserialize)]
pub struct OutboxQuery {
    page: Option<bool>,
    before: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    #[serde(rename = "@context")]
    context: String,
    #[serde(rename = "type")]
    kind: String,
    id: String,
    total_items: u64,
    first: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollectionPage {
    #[serde(rename = "@context")]
    context: String,
    #[serde(rename = "type")]
    kind: String,
    id: String,
    part_of: String,
    ordered_items: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<String>,
}

pub async fn outbox_handler(
    Path(user_id_str): Path<String>,
    Query(query): Query<OutboxQuery>,
    data: Data<FederationData>,
) -> Result<axum::response::Response, Error> {
    let uuid = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?;

    data.user_repo
        .find_by_id(uuid)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let outbox_url = format!("{}/users/{}/outbox", data.base_url, user_id_str);

    if query.page.unwrap_or(false) {
        let before: Option<DateTime<Utc>> = query.before.as_deref().and_then(|s| s.parse().ok());

        let items = data
            .object_handler
            .get_local_objects_page(uuid, before, PAGE_SIZE)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!("{}", e)))?;

        let actor_url: Url = format!("{}/users/{}", data.base_url, user_id_str)
            .parse()
            .expect("valid url");

        let has_more = items.len() == PAGE_SIZE;
        let oldest_ts = items.last().map(|(_, _, ts)| *ts);

        let followers_url = format!("{}/followers", actor_url);
        let ordered_items: Vec<serde_json::Value> = items
            .into_iter()
            .map(|(ap_id, object, _)| {
                let create_id = Url::parse(&format!("{}/activity", ap_id)).expect("valid url");
                serde_json::to_value(WithContext::new_default(CreateActivity {
                    id: create_id,
                    kind: CreateType::default(),
                    actor: ObjectId::from(actor_url.clone()),
                    object,
                    to: vec![crate::urls::AS_PUBLIC.to_string()],
                    cc: vec![followers_url.clone()],
                }))
                .expect("serializable")
            })
            .collect();

        let page_id = match &query.before {
            Some(b) => format!("{}?page=true&before={}", outbox_url, b),
            None => format!("{}?page=true", outbox_url),
        };

        let next = if has_more {
            oldest_ts.map(|ts| {
                // Use RFC 3339 with Z suffix (no + sign) to avoid percent-encoding
                let ts_str = ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                format!("{}?page=true&before={}", outbox_url, ts_str)
            })
        } else {
            None
        };

        Ok(axum::Json(OrderedCollectionPage {
            context: "https://www.w3.org/ns/activitystreams".to_string(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_id,
            part_of: outbox_url,
            ordered_items,
            next,
        })
        .into_response())
    } else {
        let total = data
            .object_handler
            .get_local_objects_for_user(uuid)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!("{}", e)))?
            .len() as u64;

        Ok(axum::Json(OrderedCollection {
            context: "https://www.w3.org/ns/activitystreams".to_string(),
            kind: "OrderedCollection".to_string(),
            id: outbox_url.clone(),
            total_items: total,
            first: format!("{}?page=true", outbox_url),
        })
        .into_response())
    }
}
