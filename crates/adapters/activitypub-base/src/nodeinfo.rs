use activitypub_federation::config::Data;
use axum::Json;
use serde::Serialize;

use crate::data::FederationData;
use crate::error::Error;

#[derive(Serialize)]
pub struct NodeInfoWellKnown {
    pub links: Vec<NodeInfoLink>,
}

#[derive(Serialize)]
pub struct NodeInfoLink {
    pub rel: String,
    pub href: String,
}

#[derive(Serialize)]
pub struct NodeInfoSoftware {
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoUsage {
    pub users: NodeInfoUsers,
    pub local_posts: u64,
}

#[derive(Serialize)]
pub struct NodeInfoUsers {
    pub total: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub version: String,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<String>,
    pub usage: NodeInfoUsage,
    pub open_registrations: bool,
}

pub async fn nodeinfo_well_known_handler(
    data: Data<FederationData>,
) -> Result<Json<NodeInfoWellKnown>, Error> {
    let href = format!("{}/nodeinfo/2.0", data.base_url);
    Ok(Json(NodeInfoWellKnown {
        links: vec![NodeInfoLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.0".to_string(),
            href,
        }],
    }))
}

pub async fn nodeinfo_handler(data: Data<FederationData>) -> Result<Json<NodeInfo>, Error> {
    let user_count = data.user_repo.count_users().await.unwrap_or(0);
    let local_posts = data.object_handler.count_local_posts().await.unwrap_or(0);

    Ok(Json(NodeInfo {
        version: "2.0".to_string(),
        software: NodeInfoSoftware {
            name: data.software_name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        protocols: vec!["activitypub".to_string()],
        usage: NodeInfoUsage {
            users: NodeInfoUsers { total: user_count },
            local_posts,
        },
        open_registrations: data.allow_registration,
    }))
}

#[cfg(test)]
#[path = "tests/nodeinfo.rs"]
mod tests;
