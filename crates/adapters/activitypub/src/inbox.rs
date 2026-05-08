use activitypub_federation::{
    axum::inbox::{receive_activity, ActivityData},
    config::Data,
    protocol::context::WithContext,
};

use crate::activities::InboxActivities;
use crate::actors::DbActor;
use crate::data::FederationData;
use crate::error::Error;

pub async fn inbox_handler(
    data: Data<FederationData>,
    activity_data: ActivityData,
) -> Result<(), Error> {
    receive_activity::<WithContext<InboxActivities>, DbActor, FederationData>(
        activity_data, &data,
    )
    .await
}
