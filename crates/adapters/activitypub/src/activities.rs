use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{AcceptType, CreateType, DeleteType, FollowType, RejectType, UndoType, UpdateType},
    traits::{Activity, Object},
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::actors::DbActor;
use crate::data::FederationData;
use crate::error::Error;
use crate::objects::{DbReview, ReviewObject};
use crate::repository::{FollowerStatus, FollowingStatus};

// --- Follow ---

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: FollowType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: ObjectId<DbActor>,
}

#[async_trait::async_trait]
impl Activity for FollowActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let target_url = self.object.inner();
        // Url::domain() strips the port, so build host:port explicitly
        let target_domain = match (target_url.host_str(), target_url.port()) {
            (Some(host), Some(port)) => format!("{}:{}", host, port),
            (Some(host), None) => host.to_string(),
            _ => return Err(Error::bad_request(anyhow::anyhow!("invalid follow target URL"))),
        };
        if target_domain != data.domain {
            return Err(Error::bad_request(anyhow::anyhow!("follow target is not a local actor")));
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let _follower = self.actor.dereference(data).await?;
        let local_actor = self.object.dereference(data).await?;

        data.federation_repo
            .add_follower(
                local_actor.user_id.clone(),
                self.actor.inner().as_str(),
                FollowerStatus::Pending,
                self.id.as_str(),
            )
            .await?;

        tracing::info!(
            follower = %self.actor.inner(),
            local_user = %local_actor.user_id.value(),
            "follow request pending approval"
        );
        Ok(())
    }
}

// --- Accept ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: AcceptType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: FollowActivity,
}

#[async_trait::async_trait]
impl Activity for AcceptActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let local_user_id = crate::urls::extract_user_id_from_url(self.object.actor.inner())
            .ok_or_else(|| Error::bad_request(anyhow::anyhow!("invalid actor URL in Follow")))?;
        let remote_actor_url = self.actor.inner().as_str();
        data.federation_repo
            .update_following_status(local_user_id, remote_actor_url, FollowingStatus::Accepted)
            .await?;
        tracing::info!(
            remote_actor = %self.actor.inner(),
            "follow accepted by remote"
        );
        Ok(())
    }
}

// --- Reject ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: RejectType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: FollowActivity,
}

#[async_trait::async_trait]
impl Activity for RejectActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        // The actor rejected our follow. Extract the local user from the original Follow's actor.
        let local_user_url = self.object.actor.inner();
        if let Some(user_id) = crate::urls::extract_user_id_from_url(local_user_url) {
            data.federation_repo
                .remove_following(user_id, self.actor.inner().as_str())
                .await?;
        }
        tracing::info!(actor = %self.actor.inner(), "follow rejected");
        Ok(())
    }
}

// --- Undo ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: UndoType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: FollowActivity,
}

#[async_trait::async_trait]
impl Activity for UndoActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        // Remote actor is unfollowing a local user
        let local_user_url = self.object.object.inner();
        if let Some(user_id) = crate::urls::extract_user_id_from_url(local_user_url) {
            data.federation_repo
                .remove_follower(user_id, self.actor.inner().as_str())
                .await?;
        }
        tracing::info!(actor = %self.actor.inner(), "unfollowed");
        Ok(())
    }
}

// --- Create ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: CreateType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: ReviewObject,
}

#[async_trait::async_trait]
impl Activity for CreateActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        if self.object.attributed_to.inner() != self.actor.inner() {
            return Err(Error::bad_request(anyhow::anyhow!(
                "activity actor does not match object attributed_to"
            )));
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        DbReview::from_json(self.object, data).await?;
        tracing::info!(actor = %self.actor.inner(), "received review");
        Ok(())
    }
}

// --- Delete ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: DeleteType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: Url,
}

#[async_trait::async_trait]
impl Activity for DeleteActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        data.federation_repo
            .delete_remote_review_by_ap_id(
                self.object.as_str(),
                self.actor.inner().as_str(),
            )
            .await?;
        tracing::info!(object = %self.object, "remote review deleted");
        Ok(())
    }
}

// --- Update ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: UpdateType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: serde_json::Value,
}

#[async_trait::async_trait]
impl Activity for UpdateActivity {
    type DataType = FederationData;
    type Error = Error;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let object: ReviewObject = match serde_json::from_value(self.object) {
            Ok(o) => o,
            Err(_) => {
                tracing::debug!(actor = %self.actor.inner(), "ignoring non-review Update activity");
                return Ok(());
            }
        };
        if object.attributed_to.inner() != self.actor.inner() {
            return Err(Error::bad_request(anyhow::anyhow!(
                "update actor does not match object attributed_to"
            )));
        }
        let ap_id = object.id.inner().as_str();
        let rating = object.rating.min(5);
        let comment = object.comment.as_deref();
        let watched_at = object.watched_at.naive_utc();
        data.federation_repo
            .update_remote_review(ap_id, self.actor.inner().as_str(), rating, comment, watched_at)
            .await?;
        tracing::info!(actor = %self.actor.inner(), ap_id = %ap_id, "remote review updated");
        Ok(())
    }
}

// --- Inbox dispatch enum ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[enum_delegate::implement(Activity)]
pub enum InboxActivities {
    #[serde(rename = "Follow")]
    Follow(FollowActivity),
    #[serde(rename = "Accept")]
    Accept(AcceptActivity),
    #[serde(rename = "Reject")]
    Reject(RejectActivity),
    #[serde(rename = "Undo")]
    Undo(UndoActivity),
    #[serde(rename = "Create")]
    Create(CreateActivity),
    #[serde(rename = "Delete")]
    Delete(DeleteActivity),
    #[serde(rename = "Update")]
    Update(UpdateActivity),
}
