use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use k_ap::{ApContentReader, ApObjectHandler};
use url::Url;

use crate::{
    goal_handler::GoalObjectHandler, review_handler::ReviewObjectHandler,
    watchlist_handler::WatchlistObjectHandler,
};

pub struct CompositeObjectHandler {
    pub review: Arc<ReviewObjectHandler>,
    pub watchlist: Arc<WatchlistObjectHandler>,
    pub goal: Arc<GoalObjectHandler>,
}

#[async_trait]
impl ApContentReader for CompositeObjectHandler {
    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        before: Option<DateTime<Utc>>,
        limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, DateTime<Utc>)>> {
        // Fetch from all three sources (watchlist/goals return all, reviews use DB pagination)
        let fetch_limit = limit * 3;
        let reviews = self
            .review
            .get_local_objects_page(user_id, before, fetch_limit)
            .await?;
        let watchlist = self
            .watchlist
            .get_local_objects_page(user_id, None, usize::MAX)
            .await?;
        let goals = self
            .goal
            .get_local_objects_page(user_id, None, usize::MAX)
            .await?;

        let mut all: Vec<(Url, serde_json::Value, DateTime<Utc>)> = Vec::new();
        all.extend(reviews);
        all.extend(watchlist);
        all.extend(goals);

        // Apply before filter and sort descending by timestamp
        if let Some(before_ts) = before {
            all.retain(|(_, _, ts)| *ts < before_ts);
        }
        all.sort_by_key(|b| std::cmp::Reverse(b.2));
        all.truncate(limit);
        Ok(all)
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        self.review.count_local_posts().await
    }
}

#[async_trait]
impl ApObjectHandler for CompositeObjectHandler {
    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let is_review = object.get("review").and_then(|v| v.as_bool()) == Some(true)
            || object.get("rating").is_some();
        let is_watchlist = object.get("watchlistEntry").and_then(|v| v.as_bool()) == Some(true)
            || (object.get("movieTitle").is_some()
                && object.get("rating").is_none()
                && object.get("review").is_none());
        let is_goal = object.get("goal").and_then(|v| v.as_bool()) == Some(true);
        if is_review {
            self.review.on_create(ap_id, actor_url, object).await
        } else if is_goal {
            self.goal.on_create(ap_id, actor_url, object).await
        } else if is_watchlist {
            self.watchlist.on_create(ap_id, actor_url, object).await
        } else {
            tracing::warn!(ap_id = %ap_id, "ignoring Create for unknown object type");
            Ok(())
        }
    }

    async fn on_update(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let is_review = object.get("review").and_then(|v| v.as_bool()) == Some(true)
            || object.get("rating").is_some();
        let is_goal = object.get("goal").and_then(|v| v.as_bool()) == Some(true);
        let is_watchlist = object.get("watchlistEntry").and_then(|v| v.as_bool()) == Some(true);
        if is_review {
            self.review.on_update(ap_id, actor_url, object).await
        } else if is_goal {
            self.goal.on_update(ap_id, actor_url, object).await
        } else if is_watchlist {
            self.watchlist.on_update(ap_id, actor_url, object).await
        } else {
            Ok(())
        }
    }

    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()> {
        self.review.on_delete(ap_id, actor_url).await?;
        self.watchlist.on_delete(ap_id, actor_url).await?;
        self.goal.on_delete(ap_id, actor_url).await?;
        Ok(())
    }

    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()> {
        self.review.on_actor_removed(actor_url).await?;
        self.watchlist.on_actor_removed(actor_url).await?;
        self.goal.on_actor_removed(actor_url).await?;
        Ok(())
    }

    async fn on_like(&self, _: &Url, _: &Url) -> anyhow::Result<()> {
        Ok(())
    }
    async fn on_announce_received(&self, _: &Url, _: &Url) -> anyhow::Result<()> {
        Ok(())
    }
    async fn on_announce_of_remote(&self, _: &Url, _: &Url) -> anyhow::Result<()> {
        Ok(())
    }
    async fn on_unlike(&self, _: &Url, _: &Url) -> anyhow::Result<()> {
        Ok(())
    }
    async fn on_mention(&self, _: &Url, _: uuid::Uuid, _: &Url) -> anyhow::Result<()> {
        Ok(())
    }
}
