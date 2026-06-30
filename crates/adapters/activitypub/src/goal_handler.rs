use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use domain::{
    models::RemoteGoalEntry,
    ports::{LocalApContentQuery, RemoteGoalRepository},
    value_objects::UserId,
};
use k_ap::{ApContentReader, ApObjectHandler};
use url::Url;

use crate::objects::{GoalObject, goal_to_ap_object};
use crate::urls::{actor_url, goal_url};

pub struct GoalObjectHandler {
    pub remote_goal_repo: Arc<dyn RemoteGoalRepository>,
    pub content_query: Arc<dyn LocalApContentQuery>,
    pub base_url: String,
}

#[async_trait]
impl ApContentReader for GoalObjectHandler {
    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        _before: Option<DateTime<chrono::Utc>>,
        _limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, DateTime<chrono::Utc>)>> {
        let uid = UserId::from_uuid(user_id);
        let goals = self
            .content_query
            .list_goals_for_user(&uid)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let actor = actor_url(&self.base_url, user_id);
        let mut results = Vec::new();
        for goal in goals {
            let ap_id = goal_url(&self.base_url, user_id, goal.year());
            let published = DateTime::from_naive_utc_and_offset(*goal.created_at(), chrono::Utc);
            let obj = goal_to_ap_object(
                ap_id.clone(),
                actor.clone(),
                goal.year(),
                goal.target_count(),
                0,
                &self.base_url,
            );
            results.push((ap_id, serde_json::to_value(obj)?, published));
        }
        Ok(results)
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
}

#[async_trait]
impl ApObjectHandler for GoalObjectHandler {
    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let obj: GoalObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(ap_id = %ap_id, "ignoring malformed goal Create: {}", e);
                return Ok(());
            }
        };
        let entry = RemoteGoalEntry {
            ap_id: ap_id.as_str().to_string(),
            actor_url: actor_url.as_str().to_string(),
            year: obj.goal_year,
            target_count: obj.goal_target,
            current_count: obj.goal_current,
            received_at: chrono::Utc::now(),
        };
        self.remote_goal_repo.save(entry).await?;
        tracing::info!(ap_id = %ap_id, year = obj.goal_year, "saved remote goal");
        Ok(())
    }

    async fn on_update(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let obj: GoalObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(ap_id = %ap_id, "ignoring malformed goal Update: {}", e);
                return Ok(());
            }
        };
        if obj.attributed_to != *actor_url {
            anyhow::bail!("goal Update actor does not match object attributed_to");
        }
        self.remote_goal_repo
            .update_by_ap_id(ap_id.as_str(), obj.goal_target, obj.goal_current)
            .await?;
        tracing::info!(ap_id = %ap_id, "updated remote goal progress");
        Ok(())
    }

    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()> {
        self.remote_goal_repo
            .remove_by_ap_id(ap_id.as_str(), actor_url.as_str())
            .await?;
        tracing::info!(ap_id = %ap_id, "removed remote goal");
        Ok(())
    }

    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()> {
        self.remote_goal_repo
            .remove_all_by_actor(actor_url.as_str())
            .await?;
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
