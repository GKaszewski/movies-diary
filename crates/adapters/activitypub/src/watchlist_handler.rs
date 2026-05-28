use std::sync::Arc;

use k_ap::ApObjectHandler;
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    models::RemoteWatchlistEntry,
    ports::{LocalApContentQuery, RemoteWatchlistRepository},
    value_objects::UserId,
};
use url::Url;

use crate::{objects::{WatchlistObject, watchlist_to_ap_object}, urls::{actor_url, watchlist_entry_url}};

pub struct WatchlistObjectHandler {
    pub remote_watchlist_repo: Arc<dyn RemoteWatchlistRepository>,
    pub content_query: Arc<dyn LocalApContentQuery>,
    pub base_url: String,
}

#[async_trait]
impl ApObjectHandler for WatchlistObjectHandler {
    async fn get_local_objects_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value)>> {
        let domain_user_id = UserId::from_uuid(user_id);
        let entries = self
            .content_query
            .get_local_watchlist_for_user(&domain_user_id)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let actor = actor_url(&self.base_url, user_id);
        let mut results = Vec::new();
        for wm in entries {
            let movie_id = wm.entry.movie_id.value();
            let ap_id = watchlist_entry_url(&self.base_url, user_id, movie_id);
            let added_at = chrono::DateTime::from_naive_utc_and_offset(wm.entry.added_at, Utc);
            let external_metadata_id = wm
                .movie
                .external_metadata_id()
                .map(|id| id.value().to_string());
            let poster_url = wm
                .movie
                .poster_path()
                .map(|p| format!("{}/images/{}", self.base_url, p.value()));
            let obj = watchlist_to_ap_object(
                ap_id.clone(),
                actor.clone(),
                wm.movie.title().value().to_string(),
                wm.movie.release_year().value(),
                external_metadata_id,
                poster_url,
                added_at,
                &self.base_url,
            );
            results.push((ap_id, serde_json::to_value(obj)?));
        }
        Ok(results)
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

    async fn on_like(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_announce_received(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_unlike(&self, _object_url: &Url, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_mention(&self, _thought_ap_id: &Url, _mentioned_user_uuid: uuid::Uuid, _actor_url: &Url) -> anyhow::Result<()> {
        Ok(())
    }
}
