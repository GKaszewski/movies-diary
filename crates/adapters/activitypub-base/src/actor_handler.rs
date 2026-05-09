use activitypub_federation::{
    axum::json::FederationJson, config::Data, protocol::context::WithContext, traits::Object,
};
use axum::extract::Path;

use crate::actors::{get_local_actor, Person};
use crate::data::FederationData;
use crate::error::Error;

pub async fn actor_handler(
    Path(user_id_str): Path<String>,
    data: Data<FederationData>,
) -> Result<FederationJson<WithContext<Person>>, Error> {
    let uuid = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| Error::bad_request(anyhow::anyhow!("invalid user id")))?;

    let db_actor = get_local_actor(uuid, &data).await?;
    let person = db_actor.into_json(&data).await?;

    Ok(FederationJson(WithContext::new_default(person)))
}
