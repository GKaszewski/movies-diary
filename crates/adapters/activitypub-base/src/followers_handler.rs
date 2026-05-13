use activitypub_federation::{axum::json::FederationJson, config::Data};
use axum::extract::{Path, Query};
use serde::Deserialize;
use serde_json::json;

use crate::data::FederationData;
use crate::error::Error;

const PAGE_SIZE: usize = 20;

#[derive(Deserialize)]
pub struct PageQuery {
    page: Option<u32>,
}

pub async fn followers_handler(
    Path(user_id_str): Path<String>,
    Query(query): Query<PageQuery>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?;

    data.user_repo
        .find_by_id(user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let collection_id = format!("{}/users/{}/followers", data.base_url, user_id_str);
    let total = data
        .federation_repo
        .count_followers(user_id)
        .await
        .map_err(Error::from)?;

    if let Some(page) = query.page {
        let page = page.max(1);
        let offset = (page.saturating_sub(1) as usize) * PAGE_SIZE;
        let followers = data
            .federation_repo
            .get_followers_page(user_id, offset as u32, PAGE_SIZE)
            .await
            .map_err(Error::from)?;

        let has_next = offset + followers.len() < total;
        let items: Vec<String> = followers.into_iter().map(|f| f.actor.url).collect();

        let mut obj = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "OrderedCollectionPage",
            "id": format!("{}?page={}", collection_id, page),
            "partOf": collection_id,
            "totalItems": total,
            "orderedItems": items,
        });

        if has_next {
            obj["next"] = json!(format!("{}?page={}", collection_id, page + 1));
        }

        Ok(FederationJson(obj))
    } else {
        Ok(FederationJson(json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "OrderedCollection",
            "id": collection_id,
            "totalItems": total,
            "first": format!("{}?page=1", collection_id),
        })))
    }
}

pub async fn following_handler(
    Path(user_id_str): Path<String>,
    Query(query): Query<PageQuery>,
    data: Data<FederationData>,
) -> Result<FederationJson<serde_json::Value>, Error> {
    let user_id = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?;

    data.user_repo
        .find_by_id(user_id)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let collection_id = format!("{}/users/{}/following", data.base_url, user_id_str);
    let total = data
        .federation_repo
        .count_following(user_id)
        .await
        .map_err(Error::from)?;

    if let Some(page) = query.page {
        let page = page.max(1);
        let offset = (page.saturating_sub(1) as usize) * PAGE_SIZE;
        let following = data
            .federation_repo
            .get_following_page(user_id, offset as u32, PAGE_SIZE)
            .await
            .map_err(Error::from)?;

        let has_next = offset + following.len() < total;
        let items: Vec<String> = following.into_iter().map(|a| a.url).collect();

        let mut obj = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "OrderedCollectionPage",
            "id": format!("{}?page={}", collection_id, page),
            "partOf": collection_id,
            "totalItems": total,
            "orderedItems": items,
        });

        if has_next {
            obj["next"] = json!(format!("{}?page={}", collection_id, page + 1));
        }

        Ok(FederationJson(obj))
    } else {
        Ok(FederationJson(json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "OrderedCollection",
            "id": collection_id,
            "totalItems": total,
            "first": format!("{}?page=1", collection_id),
        })))
    }
}
