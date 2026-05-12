use std::sync::Arc;

use activitypub_federation::{
    activity_sending::SendActivityTask,
    fetch::{object_id::ObjectId, webfinger::webfinger_resolve_actor},
    protocol::context::WithContext,
    traits::Actor,
};
use axum::{Router, routing::get, routing::post};
use url::Url;

use crate::{
    activities::{AcceptActivity, CreateActivity, FollowActivity, RejectActivity, UndoActivity, UpdateActivity},
    actors::{DbActor, get_local_actor},
    content::ApObjectHandler,
    data::FederationData,
    federation::ApFederationConfig,
    followers_handler::{followers_handler, following_handler},
    inbox::inbox_handler,
    outbox::outbox_handler,
    repository::{BlockedDomain, FederationRepository, FollowerStatus, FollowingStatus, RemoteActor},
    urls::activity_url,
    user::ApUserRepository,
    nodeinfo::{nodeinfo_handler, nodeinfo_well_known_handler},
    webfinger::webfinger_handler,
};

fn collect_inboxes(followers: &[crate::repository::Follower]) -> Vec<Url> {
    let mut seen = std::collections::HashSet::new();
    let mut inboxes = Vec::new();
    for f in followers {
        let inbox_str = f
            .actor
            .shared_inbox_url
            .as_deref()
            .unwrap_or(&f.actor.inbox_url);
        if seen.insert(inbox_str.to_string())
            && let Ok(url) = Url::parse(inbox_str) {
                inboxes.push(url);
            }
    }
    inboxes
}

pub(crate) async fn send_with_retry(
    sends: Vec<SendActivityTask>,
    data: &activitypub_federation::config::Data<FederationData>,
) -> Vec<anyhow::Error> {
    let mut failures = vec![];
    for send in sends {
        let mut delay = std::time::Duration::from_secs(1);
        for attempt in 1..=3u32 {
            match send.clone().sign_and_send(data).await {
                Ok(()) => break,
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
        user_repo: Arc<dyn ApUserRepository>,
        object_handler: Arc<dyn ApObjectHandler>,
        base_url: String,
        allow_registration: bool,
        software_name: String,
        debug: bool,
    ) -> anyhow::Result<Self> {
        let data = FederationData::new(repo, user_repo, object_handler, base_url.clone(), allow_registration, software_name);
        let federation_config = ApFederationConfig::new(data, debug).await?;
        Ok(Self {
            federation_config,
            base_url,
        })
    }

    pub fn federation_config(&self) -> &ApFederationConfig {
        &self.federation_config
    }

    pub fn request_data(&self) -> activitypub_federation::config::Data<FederationData> {
        self.federation_config.to_request_data()
    }

    pub async fn actor_json(&self, user_id_str: &str) -> anyhow::Result<String> {
        use activitypub_federation::traits::Object;
        let uuid = uuid::Uuid::parse_str(user_id_str)?;
        let data = self.federation_config.to_request_data();
        let actor = get_local_actor(uuid, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let person = actor
            .into_json(&data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_string(&WithContext::new_default(person))?)
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/.well-known/nodeinfo", get(nodeinfo_well_known_handler))
            .route("/nodeinfo/2.0", get(nodeinfo_handler))
            .route("/.well-known/webfinger", get(webfinger_handler))
            .route("/users/{id}/inbox", post(inbox_handler))
            .route("/users/{id}/outbox", get(outbox_handler))
            .route("/users/{id}/followers", get(followers_handler))
            .route("/users/{id}/following", get(following_handler))
            .layer(self.federation_config.middleware())
    }

    pub async fn follow(&self, local_user_id: uuid::Uuid, handle: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        let normalized = handle.trim_start_matches('@');
        let parts: Vec<&str> = normalized.splitn(2, '@').collect();
        if parts.len() == 2 && parts[1] == data.domain {
            return self.follow_local(local_user_id, parts[0], &data).await;
        }

        let remote_actor: DbActor = webfinger_resolve_actor(handle, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let follow_id = activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
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
            tracing::warn!(
                count = failures.len(),
                "some activity deliveries failed permanently"
            );
        }

        let remote = RemoteActor {
            url: remote_actor.ap_id.to_string(),
            handle: remote_actor.username.clone(),
            inbox_url: remote_actor.inbox_url.to_string(),
            shared_inbox_url: None,
            display_name: Some(remote_actor.username.clone()),
            avatar_url: None,
        };
        data.federation_repo
            .add_following(local_user_id, remote, &follow_id_str)
            .await?;

        Ok(())
    }

    pub async fn unfollow(
        &self,
        local_user_id: uuid::Uuid,
        actor_url_str: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        if actor_url_str.starts_with(&self.base_url) {
            return self
                .unfollow_local(local_user_id, actor_url_str, &data)
                .await;
        }

        let remote = data
            .federation_repo
            .get_remote_actor(actor_url_str)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found: {}", actor_url_str))?;

        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_ap_id = Url::parse(actor_url_str)?;
        let inbox = Url::parse(&remote.inbox_url)?;

        let follow_activity_id_str = data
            .federation_repo
            .get_follow_activity_id(local_user_id, actor_url_str)
            .await?;
        let follow_id = match follow_activity_id_str {
            Some(id) => Url::parse(&id)?,
            None => activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
        };
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: ObjectId::from(remote_ap_id),
        };

        let undo_id = activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let undo = UndoActivity {
            id: undo_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: serde_json::to_value(&follow).map_err(|e| anyhow::anyhow!("{e}"))?,
        };

        let sends = SendActivityTask::prepare(
            &WithContext::new_default(undo),
            &local_actor,
            vec![inbox],
            &data,
        )
        .await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(
                count = failures.len(),
                "some activity deliveries failed permanently"
            );
        }

        data.federation_repo
            .remove_following(local_user_id, actor_url_str)
            .await?;

        data.object_handler
            .on_actor_removed(&Url::parse(actor_url_str)?)
            .await?;

        Ok(())
    }

    pub async fn accept_follower(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_actor = data
            .federation_repo
            .get_remote_actor(remote_actor_url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found"))?;

        let follow_id_str = data
            .federation_repo
            .get_follower_follow_activity_id(local_user_id, remote_actor_url)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("follow activity id not found for {}", remote_actor_url)
            })?;
        let follow_id = Url::parse(&follow_id_str)?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(Url::parse(remote_actor_url)?),
            object: ObjectId::from(local_actor.ap_id.clone()),
        };
        let accept = AcceptActivity {
            id: activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: follow,
        };

        data.federation_repo
            .update_follower_status(local_user_id, remote_actor_url, FollowerStatus::Accepted)
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
            tracing::warn!(
                "failed to deliver Accept activity, but follower is marked accepted locally"
            );
        }

        let target_inbox = remote_actor
            .shared_inbox_url
            .clone()
            .unwrap_or_else(|| remote_actor.inbox_url.clone());
        self.spawn_backfill(local_user_id, target_inbox);

        Ok(())
    }

    pub async fn reject_follower(
        &self,
        local_user_id: uuid::Uuid,
        remote_actor_url: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let remote_actor = data
            .federation_repo
            .get_remote_actor(remote_actor_url)
            .await?
            .ok_or_else(|| anyhow::anyhow!("remote actor not found"))?;

        let follow_id = activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let follow = FollowActivity {
            id: follow_id,
            kind: Default::default(),
            actor: ObjectId::from(Url::parse(remote_actor_url)?),
            object: ObjectId::from(local_actor.ap_id.clone()),
        };
        let reject = RejectActivity {
            id: activity_url(&self.base_url).map_err(|e| anyhow::anyhow!("{e}"))?,
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
            tracing::warn!(
                count = failures.len(),
                "some activity deliveries failed permanently"
            );
        }

        data.federation_repo
            .remove_follower(local_user_id, remote_actor_url)
            .await?;

        Ok(())
    }

    pub async fn get_pending_followers(
        &self,
        local_user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        data.federation_repo
            .get_pending_followers(local_user_id)
            .await
    }

    pub async fn get_accepted_followers(
        &self,
        local_user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        let followers = data.federation_repo.get_followers(local_user_id).await?;
        Ok(followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .map(|f| f.actor)
            .collect())
    }

    pub async fn count_accepted_followers(
        &self,
        local_user_id: uuid::Uuid,
    ) -> anyhow::Result<usize> {
        let data = self.federation_config.to_request_data();
        let followers = data.federation_repo.get_followers(local_user_id).await?;
        Ok(followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .count())
    }

    pub async fn get_following(
        &self,
        local_user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.get_following(local_user_id).await
    }

    pub async fn count_following(&self, local_user_id: uuid::Uuid) -> anyhow::Result<usize> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.count_following(local_user_id).await
    }

    pub async fn remove_follower(
        &self,
        local_user_id: uuid::Uuid,
        actor_url: &str,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        data.federation_repo
            .remove_follower(local_user_id, actor_url)
            .await
    }

    /// Broadcast a single object to all accepted followers as a Create activity.
    /// Called by project-specific event handlers when new content is created.
    pub async fn broadcast_to_followers(
        &self,
        local_user_id: uuid::Uuid,
        ap_id: Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let followers = data.federation_repo.get_followers(local_user_id).await?;
        let blocked = data
            .federation_repo
            .get_blocked_actors(local_user_id)
            .await
            .unwrap_or_default();
        let blocked_set: std::collections::HashSet<String> = blocked.into_iter().collect();
        let blocked_domains = data
            .federation_repo
            .get_blocked_domains()
            .await
            .unwrap_or_default();
        let blocked_domain_set: std::collections::HashSet<String> =
            blocked_domains.into_iter().map(|d| d.domain).collect();
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .filter(|f| !blocked_set.contains(&f.actor.url))
            .filter(|f| {
                let domain = url::Url::parse(&f.actor.inbox_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                !blocked_domain_set.contains(&domain)
            })
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let create = CreateActivity {
            id: ap_id.clone(),
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };
        let create_with_ctx = WithContext::new_default(create);

        let inboxes = collect_inboxes(&accepted);

        let sends =
            SendActivityTask::prepare(&create_with_ctx, &local_actor, inboxes, &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(
                count = failures.len(),
                "some activity deliveries failed permanently"
            );
        }

        Ok(())
    }

    /// Broadcast a Delete activity to all accepted followers for a removed review.
    pub async fn broadcast_delete_to_followers(
        &self,
        local_user_id: uuid::Uuid,
        ap_id: Url,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let followers = data.federation_repo.get_followers(local_user_id).await?;
        let blocked = data
            .federation_repo
            .get_blocked_actors(local_user_id)
            .await
            .unwrap_or_default();
        let blocked_set: std::collections::HashSet<String> = blocked.into_iter().collect();
        let blocked_domains = data
            .federation_repo
            .get_blocked_domains()
            .await
            .unwrap_or_default();
        let blocked_domain_set: std::collections::HashSet<String> =
            blocked_domains.into_iter().map(|d| d.domain).collect();
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .filter(|f| !blocked_set.contains(&f.actor.url))
            .filter(|f| {
                let domain = url::Url::parse(&f.actor.inbox_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                !blocked_domain_set.contains(&domain)
            })
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let delete_id = crate::urls::activity_url(&self.base_url)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let delete = crate::activities::DeleteActivity {
            id: delete_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: ap_id,
        };
        let delete_with_ctx = WithContext::new_default(delete);
        let inboxes = collect_inboxes(&accepted);
        let sends =
            SendActivityTask::prepare(&delete_with_ctx, &local_actor, inboxes, &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(
                count = failures.len(),
                "some delete activity deliveries failed"
            );
        }
        Ok(())
    }

    /// Broadcast an Add(WatchlistObject) activity to all accepted followers.
    pub async fn broadcast_add_to_followers(
        &self,
        local_user_id: uuid::Uuid,
        ap_id: Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let followers = data.federation_repo.get_followers(local_user_id).await?;
        let blocked = data
            .federation_repo
            .get_blocked_actors(local_user_id)
            .await
            .unwrap_or_default();
        let blocked_set: std::collections::HashSet<String> = blocked.into_iter().collect();
        let blocked_domains = data
            .federation_repo
            .get_blocked_domains()
            .await
            .unwrap_or_default();
        let blocked_domain_set: std::collections::HashSet<String> =
            blocked_domains.into_iter().map(|d| d.domain).collect();
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .filter(|f| !blocked_set.contains(&f.actor.url))
            .filter(|f| {
                let domain = url::Url::parse(&f.actor.inbox_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                !blocked_domain_set.contains(&domain)
            })
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let add = crate::activities::AddActivity {
            id: ap_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };
        let add_with_ctx = WithContext::new_default(add);
        let inboxes = collect_inboxes(&accepted);
        let sends =
            SendActivityTask::prepare(&add_with_ctx, &local_actor, inboxes, &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some Add deliveries failed");
        }
        Ok(())
    }

    /// Broadcast an Undo(Add) activity to all accepted followers.
    pub async fn broadcast_undo_add_to_followers(
        &self,
        local_user_id: uuid::Uuid,
        watchlist_entry_ap_id: Url,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let followers = data.federation_repo.get_followers(local_user_id).await?;
        let blocked = data
            .federation_repo
            .get_blocked_actors(local_user_id)
            .await
            .unwrap_or_default();
        let blocked_set: std::collections::HashSet<String> = blocked.into_iter().collect();
        let blocked_domains = data
            .federation_repo
            .get_blocked_domains()
            .await
            .unwrap_or_default();
        let blocked_domain_set: std::collections::HashSet<String> =
            blocked_domains.into_iter().map(|d| d.domain).collect();
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .filter(|f| !blocked_set.contains(&f.actor.url))
            .filter(|f| {
                let domain = url::Url::parse(&f.actor.inbox_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                !blocked_domain_set.contains(&domain)
            })
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let undo_id = crate::urls::activity_url(&self.base_url)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let undo = crate::activities::UndoActivity {
            id: undo_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: serde_json::json!({
                "type": "Add",
                "id": watchlist_entry_ap_id.as_str(),
                "object": { "id": watchlist_entry_ap_id.as_str() }
            }),
        };
        let undo_with_ctx = WithContext::new_default(undo);
        let inboxes = collect_inboxes(&accepted);
        let sends =
            SendActivityTask::prepare(&undo_with_ctx, &local_actor, inboxes, &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "some Undo(Add) deliveries failed");
        }
        Ok(())
    }

    /// Broadcast an Update(Note) activity to all accepted followers for an edited review.
    pub async fn broadcast_update_to_followers(
        &self,
        local_user_id: uuid::Uuid,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let followers = data.federation_repo.get_followers(local_user_id).await?;
        let blocked = data
            .federation_repo
            .get_blocked_actors(local_user_id)
            .await
            .unwrap_or_default();
        let blocked_set: std::collections::HashSet<String> = blocked.into_iter().collect();
        let blocked_domains = data
            .federation_repo
            .get_blocked_domains()
            .await
            .unwrap_or_default();
        let blocked_domain_set: std::collections::HashSet<String> =
            blocked_domains.into_iter().map(|d| d.domain).collect();
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .filter(|f| !blocked_set.contains(&f.actor.url))
            .filter(|f| {
                let domain = url::Url::parse(&f.actor.inbox_url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                !blocked_domain_set.contains(&domain)
            })
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let update_id = Url::parse(&format!(
            "{}/activities/update/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;
        let update = crate::activities::UpdateActivity {
            id: update_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object,
        };
        let update_with_ctx = WithContext::new_default(update);
        let inboxes = collect_inboxes(&accepted);
        let sends =
            SendActivityTask::prepare(&update_with_ctx, &local_actor, inboxes, &data).await?;
        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(
                count = failures.len(),
                "some update activity deliveries failed"
            );
        }
        Ok(())
    }

    pub async fn broadcast_actor_update(&self, user_id: uuid::Uuid) -> anyhow::Result<()> {
        use activitypub_federation::traits::Object;

        let data = self.federation_config.to_request_data();
        let local_actor = get_local_actor(user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let person = local_actor.clone().into_json(&data).await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let person_json = serde_json::to_value(&person)?;

        let update_id = Url::parse(&format!(
            "{}/activities/update/{}",
            self.base_url,
            uuid::Uuid::new_v4()
        ))?;

        let update = UpdateActivity {
            id: update_id,
            kind: Default::default(),
            actor: ObjectId::from(local_actor.ap_id.clone()),
            object: person_json,
        };

        let followers = data.federation_repo.get_followers(user_id).await?;
        let accepted: Vec<_> = followers
            .into_iter()
            .filter(|f| f.status == FollowerStatus::Accepted)
            .collect();

        if accepted.is_empty() {
            return Ok(());
        }

        let inboxes = collect_inboxes(&accepted);
        let sends = SendActivityTask::prepare(
            &WithContext::new_default(update),
            &local_actor,
            inboxes,
            &data,
        )
        .await?;

        let failures = send_with_retry(sends, &data).await;
        if !failures.is_empty() {
            tracing::warn!(count = failures.len(), "actor update delivery failures");
        }
        Ok(())
    }

    pub async fn block_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();

        data.federation_repo
            .add_blocked_actor(local_user_id, actor_url)
            .await?;
        let _ = data.federation_repo.remove_follower(local_user_id, actor_url).await;
        let _ = data.federation_repo.remove_following(local_user_id, actor_url).await;

        let local_actor = get_local_actor(local_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        if let Ok(Some(remote_actor)) = data.federation_repo.get_remote_actor(actor_url).await {
            let block_id = crate::urls::activity_url(&self.base_url)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let block = crate::activities::BlockActivity {
                id: block_id,
                kind: Default::default(),
                actor: ObjectId::from(local_actor.ap_id.clone()),
                object: Url::parse(actor_url)?,
            };
            let inbox = Url::parse(&remote_actor.inbox_url)?;
            let sends = SendActivityTask::prepare(
                &WithContext::new_default(block),
                &local_actor,
                vec![inbox],
                &data,
            )
            .await?;
            let failures = send_with_retry(sends, &data).await;
            if !failures.is_empty() {
                tracing::warn!(actor = %actor_url, "failed to deliver Block activity");
            }
        }

        Ok(())
    }

    pub async fn unblock_actor(&self, local_user_id: uuid::Uuid, actor_url: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        data.federation_repo
            .remove_blocked_actor(local_user_id, actor_url)
            .await
    }

    pub async fn get_blocked_actors(&self, local_user_id: uuid::Uuid) -> anyhow::Result<Vec<RemoteActor>> {
        let data = self.federation_config.to_request_data();
        let actor_urls = data.federation_repo.get_blocked_actors(local_user_id).await?;
        let mut actors = Vec::new();
        for url in actor_urls {
            let actor = match data.federation_repo.get_remote_actor(&url).await {
                Ok(Some(a)) => a,
                _ => RemoteActor {
                    url: url.clone(),
                    handle: url.clone(),
                    inbox_url: url.clone(),
                    shared_inbox_url: None,
                    display_name: None,
                    avatar_url: None,
                },
            };
            actors.push(actor);
        }
        Ok(actors)
    }

    pub async fn add_blocked_domain(&self, domain: &str, reason: Option<&str>) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.add_blocked_domain(domain, reason).await
    }

    pub async fn remove_blocked_domain(&self, domain: &str) -> anyhow::Result<()> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.remove_blocked_domain(domain).await
    }

    pub async fn get_blocked_domains(&self) -> anyhow::Result<Vec<BlockedDomain>> {
        let data = self.federation_config.to_request_data();
        data.federation_repo.get_blocked_domains().await
    }

    async fn follow_local(
        &self,
        local_user_id: uuid::Uuid,
        target_username: &str,
        data: &activitypub_federation::config::Data<FederationData>,
    ) -> anyhow::Result<()> {
        let target = data
            .user_repo
            .find_by_username(target_username)
            .await?
            .ok_or_else(|| anyhow::anyhow!("user not found: {}", target_username))?;

        if target.id == local_user_id {
            return Err(anyhow::anyhow!("cannot follow yourself"));
        }

        let follower_actor_url = crate::urls::actor_url(&self.base_url, local_user_id).to_string();
        let target_actor_url = crate::urls::actor_url(&self.base_url, target.id);
        let target_inbox_url = format!("{}/inbox", target_actor_url);
        let follow_id = activity_url(&self.base_url)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .to_string();

        data.federation_repo
            .add_follower(
                target.id,
                &follower_actor_url,
                FollowerStatus::Accepted,
                &follow_id,
            )
            .await?;

        let target_as_remote = RemoteActor {
            url: target_actor_url.to_string(),
            handle: format!("{}@{}", target.username, data.domain),
            inbox_url: target_inbox_url,
            shared_inbox_url: None,
            display_name: Some(target.username),
            avatar_url: None,
        };
        data.federation_repo
            .add_following(local_user_id, target_as_remote, &follow_id)
            .await?;

        data.federation_repo
            .update_following_status(
                local_user_id,
                target_actor_url.as_ref(),
                FollowingStatus::Accepted,
            )
            .await?;

        tracing::info!(follower = %local_user_id, followee = %target.id, "local follow");
        Ok(())
    }

    async fn unfollow_local(
        &self,
        local_user_id: uuid::Uuid,
        target_actor_url: &str,
        data: &activitypub_federation::config::Data<FederationData>,
    ) -> anyhow::Result<()> {
        let target_url = Url::parse(target_actor_url)?;
        let target_user_id = crate::urls::extract_user_id_from_url(&target_url)
            .ok_or_else(|| anyhow::anyhow!("invalid local actor URL: {}", target_actor_url))?;

        let local_actor_url = crate::urls::actor_url(&self.base_url, local_user_id).to_string();

        data.federation_repo
            .remove_follower(target_user_id, &local_actor_url)
            .await?;
        data.federation_repo
            .remove_following(local_user_id, target_actor_url)
            .await?;

        tracing::info!(follower = %local_user_id, followee = %target_user_id, "local unfollow");
        Ok(())
    }

    fn spawn_backfill(&self, owner_user_id: uuid::Uuid, follower_inbox_url: String) {
        let config = self.federation_config.clone();
        let base_url = self.base_url.clone();
        tokio::spawn(async move {
            if let Err(e) = ActivityPubService::run_backfill(
                config,
                base_url,
                owner_user_id,
                follower_inbox_url,
            )
            .await
            {
                tracing::warn!(error = %e, "backfill: task failed");
            }
        });
    }

    async fn run_backfill(
        config: ApFederationConfig,
        base_url: String,
        owner_user_id: uuid::Uuid,
        follower_inbox_url: String,
    ) -> anyhow::Result<()> {
        const BATCH_SIZE: usize = 20;

        let data = config.to_request_data();
        let local_actor = get_local_actor(owner_user_id, &data)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let inbox = Url::parse(&follower_inbox_url)?;

        let mut objects = data
            .object_handler
            .get_local_objects_for_user(owner_user_id)
            .await?;
        objects.reverse(); // oldest first → chronological feed

        let total = objects.len();
        let mut success_count = 0usize;
        let mut failure_count = 0usize;

        for chunk in objects.chunks(BATCH_SIZE) {
            for (ap_id, object_json) in chunk {
                // Use a stable Create activity ID derived from the object's ap_id
                let create_id = Url::parse(&format!(
                    "{}/activities/create/{}",
                    base_url,
                    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, ap_id.as_str().as_bytes())
                ))?;

                let create = CreateActivity {
                    id: create_id,
                    kind: Default::default(),
                    actor: ObjectId::from(local_actor.ap_id.clone()),
                    object: object_json.clone(),
                };

                let sends = SendActivityTask::prepare(
                    &WithContext::new_default(create),
                    &local_actor,
                    vec![inbox.clone()],
                    &data,
                )
                .await?;
                let failures = send_with_retry(sends, &data).await;
                if failures.is_empty() {
                    success_count += 1;
                } else {
                    failure_count += 1;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        tracing::info!(
            user_id = %owner_user_id,
            follower = %follower_inbox_url,
            sent = success_count,
            failed = failure_count,
            total = total,
            "backfill complete"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/service.rs"]
mod tests;
