use std::sync::Arc;

use activitypub_federation::{
    activity_sending::SendActivityTask,
    config::Data,
    fetch::{object_id::ObjectId, webfinger::webfinger_resolve_actor},
    protocol::context::WithContext,
    traits::Actor,
};
use axum::{routing::get, routing::post, Router};
use domain::{ports::UserRepository, value_objects::UserId};
use url::Url;

use crate::{
    activities::{FollowActivity, UndoActivity},
    actor_handler::actor_handler,
    actors::{get_local_actor, DbActor},
    data::FederationData,
    event_handler::ActivityPubEventHandler,
    federation::ApFederationConfig,
    followers_handler::{followers_handler, following_handler},
    inbox::inbox_handler,
    outbox::outbox_handler,
    repository::{FederationRepository, RemoteActor},
    webfinger::webfinger_handler,
};

pub struct ActivityPubService {
    federation_config: ApFederationConfig,
    base_url: String,
}

impl ActivityPubService {
    pub async fn new(
        repo: Arc<dyn FederationRepository>,
        user_repo: Arc<dyn UserRepository>,
        base_url: String,
        debug: bool,
    ) -> anyhow::Result<Self> {
        let data = FederationData::new(repo, user_repo, base_url.clone());
        let federation_config = ApFederationConfig::new(data, debug).await?;
        Ok(Self {
            federation_config,
            base_url,
        })
    }

    pub fn federation_config(&self) -> &ApFederationConfig {
        &self.federation_config
    }

    pub fn request_data(&self) -> Data<FederationData> {
        self.federation_config.to_request_data()
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/.well-known/webfinger", get(webfinger_handler))
            .route("/users/{user_id}", get(actor_handler))
            .route("/users/{user_id}/inbox", post(inbox_handler))
            .route("/users/{user_id}/outbox", get(outbox_handler))
            .route("/users/{user_id}/followers", get(followers_handler))
            .route("/users/{user_id}/following", get(following_handler))
            .layer(self.federation_config.middleware())
    }

    pub fn event_handler(&self) -> ActivityPubEventHandler {
        ActivityPubEventHandler::new(self.federation_config.clone(), self.base_url.clone())
    }

    pub async fn follow(&self, local_user_id: UserId, handle: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        let remote_actor: DbActor = webfinger_resolve_actor(handle, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let local_actor = get_local_actor(local_user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let follow_id = Url::parse(&format!(
            "{}/activities/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: ObjectId::from(remote_actor.ap_id.clone()),
        };
        let follow_with_ctx = WithContext::new_default(follow);

        let sends = SendActivityTask::prepare(
            &follow_with_ctx,
            &local_actor,
            vec![remote_actor.inbox()],
            &data,
        )
        .await?;
        for send in sends {
            send.sign_and_send(&data).await?;
        }

        let remote = RemoteActor {
            url: remote_actor.ap_id.to_string(),
            handle: remote_actor
                .email
                .split('@')
                .next()
                .unwrap_or(&remote_actor.email)
                .to_string(),
            inbox_url: remote_actor.inbox_url.to_string(),
            shared_inbox_url: None,
            display_name: Some(remote_actor.email.clone()),
        };
        data.federation_repo
            .add_following(local_user_id, remote)
            .await?;

        Ok(())
    }

    pub async fn unfollow(&self, local_user_id: UserId, actor_url_str: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        let remote = data
            .federation_repo
            .get_remote_actor(actor_url_str)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found: {}", actor_url_str))?;

        let local_actor = get_local_actor(local_user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_ap_id = Url::parse(actor_url_str)?;
        let inbox = Url::parse(&remote.inbox_url)?;

        let follow_id = Url::parse(&format!(
            "{}/activities/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: ObjectId::from(remote_ap_id),
        };

        let undo_id = Url::parse(&format!(
            "{}/activities/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;
        let undo = UndoActivity {
            id: undo_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: follow,
        };
        let undo_with_ctx = WithContext::new_default(undo);

        let sends =
            SendActivityTask::prepare(&undo_with_ctx, &local_actor, vec![inbox], &data).await?;
        for send in sends {
            send.sign_and_send(&data).await?;
        }

        data.federation_repo
            .remove_following(local_user_id, actor_url_str)
            .await?;

        Ok(())
    }

    pub async fn get_following(&self, local_user_id: UserId) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.get_following(local_user_id).await
    }

    pub async fn count_following(&self, local_user_id: UserId) -> anyhow::Result<usize> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.count_following(local_user_id).await
    }
}
