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
    activities::{AcceptActivity, CreateActivity, FollowActivity, RejectActivity, UndoActivity},
    actor_handler::actor_handler,
    actors::{get_local_actor, DbActor},
    data::FederationData,
    event_handler::ActivityPubEventHandler,
    federation::ApFederationConfig,
    followers_handler::{followers_handler, following_handler},
    inbox::inbox_handler,
    outbox::outbox_handler,
    repository::{FederationRepository, FollowerStatus, RemoteActor},
    webfinger::webfinger_handler,
};

pub(crate) async fn send_with_retry(
    sends: Vec<SendActivityTask>,
    data: &Data<FederationData>,
) -> Vec<anyhow::Error> {
    let mut failures = vec![];
    for send in sends {
        let mut delay = std::time::Duration::from_secs(1);
        for attempt in 1..=3u32 {
            match send.clone().sign_and_send(data).await {
                Ok(()) => {
                    break;
                }
                Err(e) if attempt < 3 => {
                    tracing::warn!(attempt, error = %e, "delivery failed, retrying");
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(e) => {
                    tracing::error!(attempt, error = %e, "delivery failed permanently");
                    failures.push(anyhow::anyhow!(e));
                }
            }
        }
    }
    failures
}

pub struct ActivityPubService {
    federation_config: ApFederationConfig,
    base_url: String,
}

impl ActivityPubService {
    pub async fn new(
        repo: Arc<dyn FederationRepository>,
        user_repo: Arc<dyn UserRepository>,
        movie_repo: Arc<dyn domain::ports::MovieRepository>,
        base_url: String,
        debug: bool,
    ) -> anyhow::Result<Self> {
        let data = FederationData::new(repo, user_repo, movie_repo, base_url.clone());
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

    // Returns the AP actor document JSON for a local user.
    // Used for content negotiation in the HTML profile handler.
    pub async fn actor_json(&self, user_id_str: &str) -> anyhow::Result<String> {
        use activitypub_federation::traits::Object;
        use crate::actors::get_local_actor;
        let uuid = uuid::Uuid::parse_str(user_id_str)?;
        let user_id = UserId::from_uuid(uuid);
        let data = self.federation_config.to_request_data();
        let actor = get_local_actor(user_id, &data).await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let person = actor.into_json(&data).await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let with_context = WithContext::new_default(person);
        Ok(serde_json::to_string(&with_context)?)
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

        let follow_id = crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let follow_id_str = follow_id.to_string();
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
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some activity deliveries failed permanently");
        }

        let remote = RemoteActor {
            url: remote_actor.ap_id.to_string(),
            handle: remote_actor.username.clone(),
            inbox_url: remote_actor.inbox_url.to_string(),
            shared_inbox_url: None,
            display_name: Some(remote_actor.username.clone()),
        };
        data.federation_repo
            .add_following(local_user_id, remote, &follow_id_str)
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

        let follow_activity_id_str = data
            .federation_repo
            .get_follow_activity_id(local_user_id.clone(), actor_url_str)
            .await?;
        let follow_id = match follow_activity_id_str {
            Some(id) => Url::parse(&id)?,
            None => crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
        };
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: ObjectId::from(remote_ap_id),
        };

        let undo_id = crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let undo = UndoActivity {
            id: undo_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: follow,
        };
        let undo_with_ctx = WithContext::new_default(undo);

        let sends =
            SendActivityTask::prepare(&undo_with_ctx, &local_actor, vec![inbox], &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some activity deliveries failed permanently");
        }

        data.federation_repo
            .remove_following(local_user_id, actor_url_str)
            .await?;

        data.federation_repo
            .delete_remote_reviews_by_actor(actor_url_str)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

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

    pub async fn accept_follower(
        &self,
        local_user_id: UserId,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_actor = data
            .federation_repo
            .get_remote_actor(remote_actor_url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found"))?;

        let follow_id = crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(Url::parse(remote_actor_url)?),
            object: ObjectId::from(local_actor.ap_id.clone()),
        };
        let accept = AcceptActivity {
            id: crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: follow,
        };

        // Update status first so local state is correct even if delivery fails
        data.federation_repo
            .update_follower_status(local_user_id.clone(), remote_actor_url, FollowerStatus::Accepted)
            .await?;

        let inbox = Url::parse(&remote_actor.inbox_url)?;
        let sends = SendActivityTask::prepare(
            &WithContext::new_default(accept),
            &local_actor,
            vec![inbox.clone()],
            &data,
        )
        .await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!("failed to deliver Accept activity, but follower is marked accepted locally");
        }

        self.spawn_backfill(local_user_id, remote_actor.inbox_url.clone());

        Ok(())
    }

    pub async fn reject_follower(
        &self,
        local_user_id: UserId,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_actor = data
            .federation_repo
            .get_remote_actor(remote_actor_url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found"))?;

        let follow_id = crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(Url::parse(remote_actor_url)?),
            object: ObjectId::from(local_actor.ap_id.clone()),
        };
        let reject = RejectActivity {
            id: crate::urls::activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: follow,
        };

        let inbox = Url::parse(&remote_actor.inbox_url)?;
        let sends = SendActivityTask::prepare(
            &WithContext::new_default(reject),
            &local_actor,
            vec![inbox],
            &data,
        )
        .await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some activity deliveries failed permanently");
        }

        data.federation_repo
            .remove_follower(local_user_id, remote_actor_url)
            .await?;

        Ok(())
    }

    pub async fn get_pending_followers(
        &self,
        local_user_id: UserId,
    ) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        data.federation_repo
            .get_pending_followers(local_user_id)
            .await
    }

    fn spawn_backfill(&self, owner_user_id: UserId, follower_inbox_url: String) {
        let config = self.federation_config.clone();
        let base_url = self.base_url.clone();

        tokio::spawn(async move {
            if let Err(e) = ActivityPubService::run_backfill(
                config, base_url, owner_user_id, follower_inbox_url,
            ).await {
                tracing::warn!(error = %e, "backfill: task failed");
            }
        });
    }

    async fn run_backfill(
        config: ApFederationConfig,
        base_url: String,
        owner_user_id: UserId,
        follower_inbox_url: String,
    ) -> anyhow::Result<()> {
        const BATCH_SIZE: usize = 20;

        let data = config.to_request_data();
        let local_actor = get_local_actor(owner_user_id.clone(), &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let inbox = Url::parse(&follower_inbox_url)?;

        let history = data.movie_repo.get_user_history(&owner_user_id).await?;
        let local_reviews: Vec<_> = history
            .into_iter()
            .filter(|e| matches!(e.review().source(), domain::models::ReviewSource::Local))
            .collect();

        let total = local_reviews.len();

        let mut success_count = 0usize;
        let mut failure_count = 0usize;

        for chunk in local_reviews.chunks(BATCH_SIZE) {
            for entry in chunk {
                match ActivityPubService::deliver_review_to_inbox(
                    entry.review().clone(),
                    &local_actor,
                    inbox.clone(),
                    &data,
                    &base_url,
                )
                .await
                {
                    Ok(_) => success_count += 1,
                    Err(_) => failure_count += 1,
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        tracing::info!(
            user_id = %owner_user_id.value(),
            follower = %follower_inbox_url,
            sent = success_count,
            failed = failure_count,
            total = total,
            "backfill complete"
        );
        Ok(())
    }

    async fn deliver_review_to_inbox(
        review: domain::models::Review,
        local_actor: &DbActor,
        inbox: Url,
        data: &Data<FederationData>,
        base_url: &str,
    ) -> anyhow::Result<()> {
        use activitypub_federation::traits::Object;
        use crate::objects::DbReview;

        let ap_id = crate::urls::review_url(base_url, review.id());
        let db_review = DbReview { review, ap_id };
        let object = db_review.into_json(data).await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let activity_id = crate::urls::activity_url(base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let create = CreateActivity {
            id: activity_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };

        let sends = SendActivityTask::prepare(
            &WithContext::new_default(create),
            local_actor,
            vec![inbox],
            data,
        ).await?;
        let failures = send_with_retry(sends, data).await;
        if let Some(e) = failures.into_iter().next() {
            return Err(e);
        }
        Ok(())
    }
}
