use std::sync::Arc;

use activitypub_base::ApObjectHandler;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use url::Url;

use crate::{review_handler::ReviewObjectHandler, watchlist_handler::WatchlistObjectHandler};

pub struct CompositeObjectHandler {
    pub review: Arc<ReviewObjectHandler>,
    pub watchlist: Arc<WatchlistObjectHandler>,
}

#[async_trait]
impl ApObjectHandler for CompositeObjectHandler {
    async fn get_local_objects_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value)>> {
        self.review.get_local_objects_for_user(user_id).await
    }

    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        before: Option<DateTime<Utc>>,
        limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, DateTime<Utc>)>> {
        self.review.get_local_objects_page(user_id, before, limit).await
    }

    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let is_watchlist = object.get("watchlistEntry").and_then(|v| v.as_bool()) == Some(true)
            || (object.get("movieTitle").is_some() && object.get("rating").is_none());
        if object.get("rating").is_some() {
            self.review.on_create(ap_id, actor_url, object).await
        } else if is_watchlist {
            self.watchlist.on_create(ap_id, actor_url, object).await
        } else {
            tracing::debug!(ap_id = %ap_id, "ignoring Create for unknown object type");
            Ok(())
        }
    }

    async fn on_update(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        if object.get("rating").is_some() {
            self.review.on_update(ap_id, actor_url, object).await
        } else {
            Ok(())
        }
    }

    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()> {
        self.review.on_delete(ap_id, actor_url).await?;
        self.watchlist.on_delete(ap_id, actor_url).await?;
        Ok(())
    }

    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()> {
        self.review.on_actor_removed(actor_url).await?;
        self.watchlist.on_actor_removed(actor_url).await?;
        Ok(())
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        self.review.count_local_posts().await
    }
}
