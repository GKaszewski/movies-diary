use activitypub_federation::{
    config::Data,
    fetch::webfinger::{Webfinger, build_webfinger_response, extract_webfinger_name},
};
use axum::{
    extract::Query,
    http::header,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::data::FederationData;
use crate::error::Error;

#[derive(Deserialize)]
pub struct WebfingerQuery {
    resource: String,
}

pub async fn webfinger_handler(
    Query(query): Query<WebfingerQuery>,
    data: Data<FederationData>,
) -> Result<Response, Error> {
    let name = extract_webfinger_name(&query.resource, &data)?;

    let user = data
        .user_repo
        .find_by_username(name)
        .await
        .map_err(Error::from)?
        .ok_or_else(|| Error::not_found(anyhow::anyhow!("user not found")))?;

    let ap_id = crate::urls::actor_url(&data.base_url, user.id);

    let wf: Webfinger = build_webfinger_response(query.resource, ap_id);
    let body = serde_json::to_string(&wf).map_err(|e| Error::from(anyhow::anyhow!(e)))?;
    Ok(([(header::CONTENT_TYPE, "application/jrd+json")], body).into_response())
}
