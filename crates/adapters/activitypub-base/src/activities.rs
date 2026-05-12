use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{
        AcceptType, CreateType, DeleteType, FollowType, RejectType, UndoType, UpdateType,
    },
    traits::Activity,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename = "Announce")]
pub struct AnnounceType;

use crate::actors::DbActor;
use crate::data::FederationData;
use crate::error::Error;
use crate::repository::{FollowerStatus, FollowingStatus};
use domain::{events::DomainEvent, value_objects::UserId};

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
        let target_domain = match (target_url.host_str(), target_url.port()) {
            (Some(host), Some(port)) => format!("{}:{}", host, port),
            (Some(host), None) => host.to_string(),
            _ => {
                return Err(Error::bad_request(anyhow::anyhow!(
                    "invalid follow target URL"
                )));
            }
        };
        if target_domain != data.domain {
            return Err(Error::bad_request(anyhow::anyhow!(
                "follow target is not a local actor"
            )));
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let _follower = self.actor.dereference(data).await?;
        let local_actor = self.object.dereference(data).await?;

        if data.federation_repo
            .is_actor_blocked(local_actor.user_id, self.actor.inner().as_str())
            .await?
        {
            tracing::info!(actor = %self.actor.inner(), "ignoring follow from blocked actor");
            return Ok(());
        }

        data.federation_repo
            .add_follower(
                local_actor.user_id,
                self.actor.inner().as_str(),
                FollowerStatus::Pending,
                self.id.as_str(),
            )
            .await?;

        tracing::info!(
            follower = %self.actor.inner(),
            local_user = %local_actor.user_id,
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let local_user_id = crate::urls::extract_user_id_from_url(self.object.actor.inner())
            .ok_or_else(|| Error::bad_request(anyhow::anyhow!("invalid actor URL in Follow")))?;
        data.federation_repo
            .update_following_status(
                local_user_id,
                self.actor.inner().as_str(),
                FollowingStatus::Accepted,
            )
            .await?;

        if let Ok(Some(outbox_url)) = data
            .federation_repo
            .get_following_outbox_url(local_user_id, self.actor.inner().as_str())
            .await
        {
            let event = DomainEvent::FollowAccepted {
                local_user_id: UserId::from_uuid(local_user_id),
                remote_actor_url: self.actor.inner().to_string(),
                outbox_url,
            };
            if let Some(publisher) = &data.event_publisher {
                if let Err(e) = publisher.publish(&event).await {
                    tracing::warn!(error = %e, "failed to publish FollowAccepted event");
                }
            }
        }

        tracing::info!(remote_actor = %self.actor.inner(), "follow accepted by remote");
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        if let Some(user_id) = crate::urls::extract_user_id_from_url(self.object.actor.inner()) {
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
    pub(crate) object: serde_json::Value,
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring Undo from blocked domain");
            return Ok(());
        }

        let obj_type = self.object.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match obj_type {
            "Follow" => {
                if let Some(obj_url) = self.object.get("object").and_then(|o| o.as_str()) {
                    if let Ok(url) = Url::parse(obj_url) {
                        if let Some(user_id) = crate::urls::extract_user_id_from_url(&url) {
                            data.federation_repo
                                .remove_follower(user_id, self.actor.inner().as_str())
                                .await?;
                        }
                    }
                }
                data.object_handler
                    .on_actor_removed(self.actor.inner())
                    .await
                    .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
                tracing::info!(actor = %self.actor.inner(), "unfollowed");
            }
            "Add" => {
                let ap_id_str = self.object
                    .get("object")
                    .and_then(|o| o.get("id"))
                    .and_then(|id| id.as_str())
                    .or_else(|| self.object.get("id").and_then(|id| id.as_str()));

                if let Some(ap_id_str) = ap_id_str {
                    if let Ok(ap_id) = Url::parse(ap_id_str) {
                        data.object_handler
                            .on_delete(&ap_id, self.actor.inner())
                            .await
                            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
                        tracing::info!(ap_id = %ap_id_str, "undo Add (watchlist remove)");
                    }
                }
            }
            other => {
                tracing::debug!(kind = %other, "ignoring Undo of unknown activity type");
            }
        }

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
    pub(crate) object: serde_json::Value,
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
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let ap_id = self.id.clone();
        let actor_url = self.actor.inner().clone();
        data.object_handler
            .on_create(&ap_id, &actor_url, self.object)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        tracing::info!(actor = %actor_url, "received create activity");
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let actor_url = self.actor.inner().clone();
        data.object_handler
            .on_delete(&self.object, &actor_url)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        tracing::info!(object = %self.object, "received delete activity");
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let ap_id = self.id.clone();
        let actor_url = self.actor.inner().clone();
        data.object_handler
            .on_update(&ap_id, &actor_url, self.object)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        tracing::info!(actor = %actor_url, "received update activity");
        Ok(())
    }
}

// --- Announce ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: AnnounceType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: Url,
    pub(crate) published: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait::async_trait]
impl Activity for AnnounceActivity {
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        let object_domain = self.object.host_str().unwrap_or("");
        if object_domain != data.domain {
            return Ok(());
        }
        data.federation_repo
            .add_announce(
                self.id.as_str(),
                self.object.as_str(),
                self.actor.inner().as_str(),
                self.published.unwrap_or_else(chrono::Utc::now),
            )
            .await?;
        tracing::info!(actor = %self.actor.inner(), object = %self.object, "received announce");
        Ok(())
    }
}

// --- Add ---

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename = "Add")]
pub struct AddType;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: AddType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: serde_json::Value,
}

#[async_trait::async_trait]
impl Activity for AddActivity {
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring Add from blocked domain");
            return Ok(());
        }
        let ap_id = self.id.clone();
        let actor_url = self.actor.inner().clone();
        data.object_handler
            .on_create(&ap_id, &actor_url, self.object)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        tracing::info!(actor = %actor_url, "received Add activity");
        Ok(())
    }
}

// --- Block ---

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename = "Block")]
pub struct BlockType;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockActivity {
    pub(crate) id: Url,
    #[serde(rename = "type", default)]
    pub(crate) kind: BlockType,
    pub(crate) actor: ObjectId<DbActor>,
    pub(crate) object: Url,
}

#[async_trait::async_trait]
impl Activity for BlockActivity {
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
        let domain = self.actor().host_str().unwrap_or("");
        if data.federation_repo.is_domain_blocked(domain).await? {
            tracing::info!(actor = %self.actor(), "ignoring activity from blocked domain");
            return Ok(());
        }
        // They blocked us — remove them from our following list
        if let Some(local_user_id) = crate::urls::extract_user_id_from_url(&self.object) {
            let _ = data
                .federation_repo
                .remove_following(local_user_id, self.actor.inner().as_str())
                .await;
        }
        tracing::info!(actor = %self.actor.inner(), "received block");
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
    #[serde(rename = "Announce")]
    Announce(AnnounceActivity),
    #[serde(rename = "Add")]
    Add(AddActivity),
    #[serde(rename = "Block")]
    Block(BlockActivity),
}
