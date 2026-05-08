use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::activity::{AcceptType, CreateType, DeleteType, FollowType, RejectType, UndoType},
    traits::{Activity, Actor, Object},
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::actors::DbActor;
use crate::data::FederationData;
use crate::error::Error;
use crate::objects::{DbReview, ReviewObject};
use crate::repository::FollowerStatus;

// --- Follow ---

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowActivity {
    pub(crate) id: Url,
    #[serde(rename = "type")]
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
        // Verify the target is a local actor
        let target_url = self.object.inner();
        if target_url.domain() != Some(&data.domain) {
            return Err(Error(anyhow::anyhow!(
                "follow target is not a local actor"
            )));
        }
        Ok(())
    }

    async fn receive(self, data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let follower = self.actor.dereference(data).await?;
        let local_actor = self.object.dereference(data).await?;

        data.federation_repo
            .add_follower(
                local_actor.user_id.clone(),
                self.actor.inner().as_str(),
                FollowerStatus::Accepted,
            )
            .await?;

        // Send Accept back
        let accept_id =
            Url::parse(&format!("{}/activities/{}", data.base_url, uuid::Uuid::new_v4()))
                .expect("valid url");
        let accept = AcceptActivity {
            id: accept_id,
            kind: Default::default(),
            actor: self.object.clone(),
            object: self.clone(),
        };

        use activitypub_federation::activity_sending::SendActivityTask;
        use activitypub_federation::protocol::context::WithContext;

        let accept_with_ctx = WithContext::new_default(accept);
        let sends =
            SendActivityTask::prepare(&accept_with_ctx, &local_actor, vec![follower.inbox()], data)
                .await?;
        for send in sends {
            send.sign_and_send(data).await?;
        }

        tracing::info!(
            follower = %self.actor.inner(),
            local_user = %local_actor.user_id.value(),
            "accepted follow"
        );

        Ok(())
    }
}

// --- Accept ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptActivity {
    pub(crate) id: Url,
    #[serde(rename = "type")]
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

    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        let remote_actor_url = self.actor.into_inner().to_string();
        tracing::info!(remote_actor_url = %remote_actor_url, "Follow accepted by remote instance");
        // TODO(ap): update ap_following to track accepted status
        Ok(())
    }
}

// --- Reject ---

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectActivity {
    pub(crate) id: Url,
    #[serde(rename = "type")]
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
        let path = local_user_url.path();
        if let Some(uid_str) = path.strip_prefix("/users/").and_then(|s| s.split('/').next()) {
            if let Ok(uuid) = uuid::Uuid::parse_str(uid_str) {
                let user_id = domain::value_objects::UserId::from_uuid(uuid);
                data.federation_repo
                    .remove_following(user_id, self.actor.inner().as_str())
                    .await?;
            }
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
    #[serde(rename = "type")]
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
        let path = local_user_url.path();
        if let Some(uid_str) = path.strip_prefix("/users/").and_then(|s| s.split('/').next()) {
            if let Ok(uuid) = uuid::Uuid::parse_str(uid_str) {
                let user_id = domain::value_objects::UserId::from_uuid(uuid);
                data.federation_repo
                    .remove_follower(user_id, self.actor.inner().as_str())
                    .await?;
            }
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
    #[serde(rename = "type")]
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
    #[serde(rename = "type")]
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

    async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        tracing::info!(actor = %self.actor.inner(), object = %self.object, "delete received (no-op)");
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
}
