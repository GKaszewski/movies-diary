use std::time::Duration;

use async_trait::async_trait;
use chrono::Datelike;
use domain::{errors::DomainError, events::DomainEvent, ports::PeriodicJob};

use crate::context::AppContext;

pub struct ImportSessionCleanupJob {
    ctx: AppContext,
}

impl ImportSessionCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for ImportSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::import::cleanup::execute(&self.ctx).await?;
        tracing::info!("import session cleanup: removed {} expired sessions", n);
        Ok(())
    }
}

pub struct WatchEventCleanupJob {
    ctx: AppContext,
}

impl WatchEventCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for WatchEventCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = crate::integrations::cleanup::execute(&self.ctx).await?;
        if n > 0 {
            tracing::info!("watch event cleanup: removed {n} old entries");
        }
        Ok(())
    }
}

pub struct EnrichmentStalenessJob {
    ctx: AppContext,
}

impl EnrichmentStalenessJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for EnrichmentStalenessJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(3600)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let stale = self.ctx.repos.movie_profile.list_stale().await?;
        if stale.is_empty() {
            return Ok(());
        }
        tracing::info!("enrichment scan: {} stale movies", stale.len());
        for (movie_id, external_metadata_id) in stale {
            let event = DomainEvent::MovieEnrichmentRequested {
                movie_id,
                external_metadata_id,
            };
            self.ctx.services.event_publisher.publish(&event).await?;
        }
        Ok(())
    }
}

pub struct WrapUpAutoGenerateJob {
    ctx: AppContext,
}

impl WrapUpAutoGenerateJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for WrapUpAutoGenerateJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let now = chrono::Utc::now().naive_utc();
        // Only run in January
        if now.month() != 1 {
            return Ok(());
        }
        let year = now.year() - 1;
        let start = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
            .ok_or_else(|| DomainError::ValidationError("invalid date".into()))?;
        let end = chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| DomainError::ValidationError("invalid date".into()))?;

        let users = self.ctx.repos.user.list_with_stats().await?;
        for user in &users {
            if user.total_movies > 0 {
                let existing = self
                    .ctx
                    .repos
                    .wrapup_repo
                    .find_existing(Some(user.user_id.value()), start, end)
                    .await?;
                if existing.is_none() {
                    let cmd = crate::wrapup::commands::RequestWrapUpCommand {
                        user_id: Some(user.user_id.value()),
                        start_date: start,
                        end_date: end,
                    };
                    if let Err(e) = crate::wrapup::generate::execute(&self.ctx, cmd).await {
                        tracing::warn!(
                            "auto-generate wrapup for user {} failed: {e}",
                            user.user_id.value()
                        );
                    }
                }
            }
        }

        // Global wrap-up
        let existing = self
            .ctx
            .repos
            .wrapup_repo
            .find_existing(None, start, end)
            .await?;
        if existing.is_none() {
            let cmd = crate::wrapup::commands::RequestWrapUpCommand {
                user_id: None,
                start_date: start,
                end_date: end,
            };
            if let Err(e) = crate::wrapup::generate::execute(&self.ctx, cmd).await {
                tracing::warn!("auto-generate global wrapup failed: {e}");
            }
        }

        Ok(())
    }
}

pub struct RefreshSessionCleanupJob {
    ctx: AppContext,
}

impl RefreshSessionCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for RefreshSessionCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let n = self.ctx.repos.refresh_session.delete_expired().await?;
        if n > 0 {
            tracing::info!("refresh session cleanup: removed {n} expired sessions");
        }
        Ok(())
    }
}

pub struct WrapUpCleanupJob {
    ctx: AppContext,
}

impl WrapUpCleanupJob {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl PeriodicJob for WrapUpCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
        let n = self
            .ctx
            .repos
            .wrapup_repo
            .delete_failed_older_than(cutoff)
            .await?;
        if n > 0 {
            tracing::info!("wrapup cleanup: removed {n} failed records");
        }
        Ok(())
    }
}
