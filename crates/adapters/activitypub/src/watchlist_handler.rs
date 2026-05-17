use std::sync::Arc;

use k_ap::ApObjectHandler;
use async_trait::async_trait;
use chrono::Utc;
use domain::{models::RemoteWatchlistEntry, ports::RemoteWatchlistRepository};
use url::Url;

use crate::objects::WatchlistObject;

pub struct WatchlistObjectHandler {
    pub remote_watchlist_repo: Arc<dyn RemoteWatchlistRepository>,
}

#[async_trait]
impl ApObjectHandler for WatchlistObjectHandler {
    async fn get_local_objects_for_user(
        &self,
        _user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value)>> {
        Ok(vec![])
    }

    async fn get_local_objects_page(
        &self,
        _user_id: uuid::Uuid,
        _before: Option<chrono::DateTime<Utc>>,
        _limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, chrono::DateTime<Utc>)>> {
        Ok(vec![])
    }

    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let obj: WatchlistObject = serde_json::from_value(object)?;
        let added_at = obj.published;
        let entry = RemoteWatchlistEntry {
            ap_id: ap_id.as_str().to_string(),
            actor_url: actor_url.as_str().to_string(),
            movie_title: obj.movie_title,
            release_year: obj.release_year,
            external_metadata_id: obj.external_metadata_id,
            poster_url: obj.poster_url,
            added_at,
        };
        self.remote_watchlist_repo.save(entry).await?;
        tracing::info!(ap_id = %ap_id, "saved remote watchlist entry");
        Ok(())
    }

    async fn on_update(
        &self,
        _ap_id: &Url,
        _actor_url: &Url,
        _object: serde_json::Value,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_delete(&self, ap_id: &Url, actor_url: &Url) -> anyhow::Result<()> {
        self.remote_watchlist_repo
            .remove_by_ap_id(ap_id.as_str(), actor_url.as_str())
            .await?;
        tracing::info!(ap_id = %ap_id, "removed remote watchlist entry");
        Ok(())
    }

    async fn on_actor_removed(&self, actor_url: &Url) -> anyhow::Result<()> {
        self.remote_watchlist_repo
            .remove_all_by_actor(actor_url.as_str())
            .await?;
        Ok(())
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
}
