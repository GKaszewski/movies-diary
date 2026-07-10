use async_trait::async_trait;
use chrono::Datelike;
use domain::ports::EventHandler;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{
        GoalRepository, LocalApContentQuery, MovieRepository, ReviewRepository, StatsRepository,
        UserFederationSettingsQuery,
    },
    value_objects::{MovieId, ReviewId, UserId},
};
use std::sync::Arc;

use k_ap::{ActivityPubService, ApVisibility};

use crate::objects::{ReviewApInput, goal_to_ap_object, review_to_ap_object};
use crate::urls::{actor_url, goal_url, review_url};

pub struct ActivityPubEventHandler {
    ap_service: Arc<ActivityPubService>,
    content_query: Arc<dyn LocalApContentQuery>,
    review_repo: Arc<dyn ReviewRepository>,
    movie_repo: Arc<dyn MovieRepository>,
    goal_repo: Arc<dyn GoalRepository>,
    stats_repo: Arc<dyn StatsRepository>,
    federation_settings: Arc<dyn UserFederationSettingsQuery>,
    base_url: String,
}

impl ActivityPubEventHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ap_service: Arc<ActivityPubService>,
        content_query: Arc<dyn LocalApContentQuery>,
        review_repo: Arc<dyn ReviewRepository>,
        movie_repo: Arc<dyn MovieRepository>,
        goal_repo: Arc<dyn GoalRepository>,
        stats_repo: Arc<dyn StatsRepository>,
        federation_settings: Arc<dyn UserFederationSettingsQuery>,
        base_url: String,
    ) -> Self {
        Self {
            ap_service,
            content_query,
            review_repo,
            movie_repo,
            goal_repo,
            stats_repo,
            federation_settings,
            base_url,
        }
    }
}

#[async_trait]
impl EventHandler for ActivityPubEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::ReviewLogged {
                review_id, user_id, ..
            } => self
                .on_review_logged(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::ReviewUpdated {
                review_id, user_id, ..
            } => self
                .on_review_updated(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::ReviewDeleted { review_id, user_id } => self
                .on_review_deleted(user_id, review_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::UserUpdated { user_id } => self
                .ap_service
                .broadcast_actor_update(user_id.value())
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::WatchlistEntryAdded {
                user_id,
                movie_id,
                movie_title,
                release_year,
                external_metadata_id,
                added_at,
            } => self
                .on_watchlist_added(
                    user_id,
                    movie_id,
                    movie_title,
                    *release_year,
                    external_metadata_id,
                    added_at,
                )
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::WatchlistEntryRemoved { user_id, movie_id } => self
                .on_watchlist_removed(user_id, movie_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::FederationDeliveryRequested {
                inbox_url,
                activity_json,
                signing_actor_id,
            } => {
                let inbox: url::Url = inbox_url
                    .parse()
                    .map_err(|e| DomainError::InfrastructureError(format!("bad inbox URL: {e}")))?;
                let activity: serde_json::Value =
                    serde_json::from_str(activity_json).map_err(|e| {
                        DomainError::InfrastructureError(format!("bad activity JSON: {e}"))
                    })?;
                self.ap_service
                    .deliver_to_inbox(inbox, activity, *signing_actor_id)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))
            }
            DomainEvent::PosterSynced { movie_id } => self
                .on_poster_synced(movie_id)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::GoalCreated {
                user_id,
                year,
                target_count,
                ..
            } => self
                .broadcast_goal(user_id, *year, *target_count, true)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::GoalUpdated {
                user_id,
                year,
                target_count,
                ..
            } => self
                .broadcast_goal(user_id, *year, *target_count, false)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::GoalDeleted { user_id, year, .. } => self
                .on_goal_deleted(user_id, *year)
                .await
                .map_err(|e| DomainError::InfrastructureError(e.to_string())),
            DomainEvent::UserDeleted { user_id } => {
                let ap_id = actor_url(&self.base_url, user_id.value());
                self.ap_service
                    .broadcast_delete_to_followers(user_id.value(), ap_id)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))
            }
            DomainEvent::UserAccountMoved {
                user_id,
                new_actor_url,
            } => {
                let target = new_actor_url.parse::<url::Url>().map_err(|e| {
                    DomainError::InfrastructureError(format!("invalid new_actor_url: {e}"))
                })?;
                self.ap_service
                    .broadcast_move(user_id.value(), target)
                    .await
                    .map_err(|e| DomainError::InfrastructureError(e.to_string()))
            }
            _ => Ok(()),
        }
    }
}

impl ActivityPubEventHandler {
    async fn on_review_logged(&self, user_id: &UserId, review_id: &ReviewId) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.reviews {
            return Ok(());
        }

        let review = match self.review_repo.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let ap_id = review_url(&self.base_url, review_id);
        let actor = actor_url(&self.base_url, user_id.value());

        let movie = self
            .movie_repo
            .get_movie_by_id(review.movie_id())
            .await
            .ok()
            .flatten();
        let movie_title = movie
            .as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie
            .as_ref()
            .map(|m| m.release_year().value())
            .unwrap_or(0);
        let obj = review_to_ap_object(
            &review,
            ReviewApInput {
                ap_id: ap_id.clone(),
                actor_url: actor,
                movie_title,
                release_year,
                external_metadata_id: movie
                    .as_ref()
                    .and_then(|m| m.external_metadata_id())
                    .map(|id| id.value().to_string()),
                poster_url: movie
                    .as_ref()
                    .and_then(|m| m.poster_path())
                    .map(|p| format!("{}/images/{}", self.base_url, p.value())),
                base_url: self.base_url.clone(),
            },
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_create_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;

        let year = review.watched_at().year() as u16;
        self.broadcast_goal_progress_update(user_id, year).await?;

        Ok(())
    }

    async fn on_review_updated(
        &self,
        user_id: &UserId,
        review_id: &ReviewId,
    ) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.reviews {
            return Ok(());
        }

        let review = match self.review_repo.get_review_by_id(review_id).await? {
            Some(r) => r,
            None => return Ok(()),
        };

        let ap_id = review_url(&self.base_url, review_id);
        let actor = actor_url(&self.base_url, user_id.value());

        let movie = self
            .movie_repo
            .get_movie_by_id(review.movie_id())
            .await
            .ok()
            .flatten();
        let movie_title = movie
            .as_ref()
            .map(|m| m.title().value().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let release_year = movie
            .as_ref()
            .map(|m| m.release_year().value())
            .unwrap_or(0);
        let obj = review_to_ap_object(
            &review,
            ReviewApInput {
                ap_id,
                actor_url: actor,
                movie_title,
                release_year,
                external_metadata_id: movie
                    .as_ref()
                    .and_then(|m| m.external_metadata_id())
                    .map(|id| id.value().to_string()),
                poster_url: movie
                    .as_ref()
                    .and_then(|m| m.poster_path())
                    .map(|p| format!("{}/images/{}", self.base_url, p.value())),
                base_url: self.base_url.clone(),
            },
        );
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_update_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;

        Ok(())
    }

    async fn on_review_deleted(
        &self,
        user_id: &UserId,
        review_id: &ReviewId,
    ) -> anyhow::Result<()> {
        let ap_id = review_url(&self.base_url, review_id);
        self.ap_service
            .broadcast_delete_to_followers(user_id.value(), ap_id)
            .await?;
        Ok(())
    }

    async fn on_watchlist_added(
        &self,
        user_id: &UserId,
        movie_id: &domain::value_objects::MovieId,
        movie_title: &str,
        release_year: u16,
        external_metadata_id: &Option<String>,
        added_at: &chrono::NaiveDateTime,
    ) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.watchlist {
            return Ok(());
        }

        use crate::urls::watchlist_entry_url;
        let ap_id = watchlist_entry_url(&self.base_url, user_id.value(), movie_id.value());
        let actor = actor_url(&self.base_url, user_id.value());

        let poster_url = self
            .movie_repo
            .get_movie_by_id(movie_id)
            .await
            .ok()
            .flatten()
            .and_then(|m| {
                m.poster_path()
                    .map(|p| format!("{}/images/{}", self.base_url, p.value()))
            });

        let added_at_utc =
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(*added_at, chrono::Utc);
        let obj = crate::objects::watchlist_to_ap_object(crate::objects::WatchlistApInput {
            ap_id: ap_id.clone(),
            actor_url: actor,
            movie_title: movie_title.to_string(),
            release_year,
            external_metadata_id: external_metadata_id.clone(),
            poster_url,
            added_at: added_at_utc,
            base_url: self.base_url.clone(),
        });
        let json = serde_json::to_value(obj)?;

        self.ap_service
            .broadcast_create_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;
        Ok(())
    }

    async fn on_watchlist_removed(
        &self,
        user_id: &UserId,
        movie_id: &domain::value_objects::MovieId,
    ) -> anyhow::Result<()> {
        use crate::urls::watchlist_entry_url;
        let ap_id = watchlist_entry_url(&self.base_url, user_id.value(), movie_id.value());
        self.ap_service
            .broadcast_delete_to_followers(user_id.value(), ap_id)
            .await?;
        Ok(())
    }

    async fn on_poster_synced(&self, movie_id: &MovieId) -> anyhow::Result<()> {
        let entries = self
            .content_query
            .get_local_reviews_for_movie(movie_id)
            .await?;

        let movie = self.movie_repo.get_movie_by_id(movie_id).await?;
        let movie = match movie {
            Some(m) => m,
            None => return Ok(()),
        };
        let external_metadata_id = movie
            .external_metadata_id()
            .map(|id| id.value().to_string());
        let poster_url = movie
            .poster_path()
            .map(|p| format!("{}/images/{}", self.base_url, p.value()));

        for entry in entries {
            let review = entry.review();
            let user_id = review.user_id();

            let flags = self
                .federation_settings
                .get_federation_flags(user_id)
                .await
                .unwrap_or_default();
            if !flags.reviews {
                continue;
            }

            let ap_id = review_url(&self.base_url, review.id());
            let actor = actor_url(&self.base_url, user_id.value());

            let obj = review_to_ap_object(
                review,
                ReviewApInput {
                    ap_id,
                    actor_url: actor,
                    movie_title: movie.title().value().to_string(),
                    release_year: movie.release_year().value(),
                    external_metadata_id: external_metadata_id.clone(),
                    poster_url: poster_url.clone(),
                    base_url: self.base_url.clone(),
                },
            );
            let json = serde_json::to_value(obj)?;

            self.ap_service
                .broadcast_update_note(user_id.value(), json, ApVisibility::Public, vec![])
                .await?;
        }

        Ok(())
    }

    async fn broadcast_goal_progress_update(
        &self,
        user_id: &UserId,
        year: u16,
    ) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.goals {
            return Ok(());
        }
        let Some(goal) = self
            .goal_repo
            .find_by_user_and_year(user_id, year)
            .await
            .ok()
            .flatten()
        else {
            return Ok(());
        };
        let current = self
            .stats_repo
            .count_reviews_in_year(user_id, year)
            .await
            .unwrap_or(0);
        let ap_id = goal_url(&self.base_url, user_id.value(), year);
        let actor = actor_url(&self.base_url, user_id.value());
        let obj = goal_to_ap_object(
            ap_id,
            actor,
            year,
            goal.target_count(),
            current,
            &self.base_url,
        );
        let json = serde_json::to_value(obj)?;
        self.ap_service
            .broadcast_update_note(user_id.value(), json, ApVisibility::Public, vec![])
            .await?;
        Ok(())
    }

    async fn broadcast_goal(
        &self,
        user_id: &UserId,
        year: u16,
        target_count: u32,
        is_create: bool,
    ) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.goals {
            return Ok(());
        }
        let current = self
            .stats_repo
            .count_reviews_in_year(user_id, year)
            .await
            .unwrap_or(0);

        let ap_id = goal_url(&self.base_url, user_id.value(), year);
        let actor = actor_url(&self.base_url, user_id.value());
        let obj = goal_to_ap_object(ap_id, actor, year, target_count, current, &self.base_url);
        let json = serde_json::to_value(obj)?;
        if is_create {
            self.ap_service
                .broadcast_create_note(user_id.value(), json, ApVisibility::Public, vec![])
                .await?;
        } else {
            self.ap_service
                .broadcast_update_note(user_id.value(), json, ApVisibility::Public, vec![])
                .await?;
        }
        Ok(())
    }

    async fn on_goal_deleted(&self, user_id: &UserId, year: u16) -> anyhow::Result<()> {
        let flags = self
            .federation_settings
            .get_federation_flags(user_id)
            .await
            .unwrap_or_default();
        if !flags.goals {
            return Ok(());
        }
        let ap_id = goal_url(&self.base_url, user_id.value(), year);
        self.ap_service
            .broadcast_delete_to_followers(user_id.value(), ap_id)
            .await?;
        Ok(())
    }
}
