use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use domain::{
    models::{RemoteWatchlistEntry, WatchlistWithMovie},
    ports::{LocalApContentQuery, RemoteWatchlistRepository},
    value_objects::UserId,
};
use k_ap::{ApContentReader, ApObjectHandler};
use url::Url;

use crate::objects::{WatchlistApInput, WatchlistObject, watchlist_to_ap_object};
use crate::urls::{actor_url, watchlist_entry_url};

pub struct WatchlistObjectHandler {
    pub remote_watchlist_repo: Arc<dyn RemoteWatchlistRepository>,
    pub content_query: Arc<dyn LocalApContentQuery>,
    pub base_url: String,
}

#[async_trait]
impl ApContentReader for WatchlistObjectHandler {
    async fn get_local_objects_page(
        &self,
        user_id: uuid::Uuid,
        _before: Option<DateTime<chrono::Utc>>,
        _limit: usize,
    ) -> anyhow::Result<Vec<(Url, serde_json::Value, DateTime<chrono::Utc>)>> {
        let uid = UserId::from_uuid(user_id);
        let entries = self
            .content_query
            .get_local_watchlist_for_user(&uid)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let actor = actor_url(&self.base_url, user_id);
        let mut results = Vec::new();
        for WatchlistWithMovie { entry, movie } in entries {
            let ap_id = watchlist_entry_url(&self.base_url, user_id, entry.movie_id.value());
            let published = DateTime::from_naive_utc_and_offset(entry.added_at, chrono::Utc);
            let poster_url = movie
                .poster_path()
                .map(|p| format!("{}/images/{}", self.base_url, p.value()));
            let obj = watchlist_to_ap_object(WatchlistApInput {
                ap_id: ap_id.clone(),
                actor_url: actor.clone(),
                movie_title: movie.title().value().to_string(),
                release_year: movie.release_year().value(),
                external_metadata_id: movie
                    .external_metadata_id()
                    .map(|id| id.value().to_string()),
                poster_url,
                added_at: published,
                base_url: self.base_url.clone(),
            });
            results.push((ap_id, serde_json::to_value(obj)?, published));
        }
        Ok(results)
    }

    async fn count_local_posts(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
}

#[async_trait]
impl ApObjectHandler for WatchlistObjectHandler {
    async fn on_create(
        &self,
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let mut obj: WatchlistObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(ap_id = %ap_id, "ignoring malformed watchlist Create: {}", e);
                return Ok(());
            }
        };
        obj.movie_title = ammonia::clean(&obj.movie_title);
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
        ap_id: &Url,
        actor_url: &Url,
        object: serde_json::Value,
    ) -> anyhow::Result<()> {
        let mut obj: WatchlistObject = match serde_json::from_value(object) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(ap_id = %ap_id, "ignoring malformed watchlist Update: {}", e);
                return Ok(());
            }
        };
        if obj.attributed_to != *actor_url {
            anyhow::bail!("watchlist Update actor does not match object attributed_to");
        }
        obj.movie_title = ammonia::clean(&obj.movie_title);
        let entry = RemoteWatchlistEntry {
            ap_id: ap_id.as_str().to_string(),
            actor_url: actor_url.as_str().to_string(),
            movie_title: obj.movie_title,
            release_year: obj.release_year,
            external_metadata_id: obj.external_metadata_id,
            poster_url: obj.poster_url,
            added_at: obj.published,
        };
        self.remote_watchlist_repo.save(entry).await?;
        tracing::info!(ap_id = %ap_id, "updated remote watchlist entry");
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
