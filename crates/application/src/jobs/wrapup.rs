use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Datelike;
use domain::{
    errors::DomainError,
    ports::{EventPublisher, PeriodicJob, UserRepository, WrapUpRepository},
};

pub struct WrapUpAutoGenerateJob {
    user: Arc<dyn UserRepository>,
    wrapup_repo: Arc<dyn WrapUpRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl WrapUpAutoGenerateJob {
    pub fn new(
        user: Arc<dyn UserRepository>,
        wrapup_repo: Arc<dyn WrapUpRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user,
            wrapup_repo,
            event_publisher,
        }
    }
}

#[async_trait]
impl PeriodicJob for WrapUpAutoGenerateJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let now = chrono::Utc::now().naive_utc();
        if now.month() != 1 {
            return Ok(());
        }
        let year = now.year() - 1;
        let start = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
            .ok_or_else(|| DomainError::ValidationError("invalid date".into()))?;
        let end = chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| DomainError::ValidationError("invalid date".into()))?;

        let users = self.user.list_with_stats().await?;
        for user in &users {
            if user.total_movies > 0 {
                let existing = self
                    .wrapup_repo
                    .find_existing(Some(user.user_id.value()), start, end)
                    .await?;
                if existing.is_none() {
                    let cmd = crate::wrapup::commands::RequestWrapUpCommand {
                        user_id: Some(user.user_id.value()),
                        start_date: start,
                        end_date: end,
                    };
                    if let Err(e) = crate::wrapup::generate::execute(
                        self.wrapup_repo.clone(),
                        self.event_publisher.clone(),
                        cmd,
                    )
                    .await
                    {
                        tracing::warn!(
                            "auto-generate wrapup for user {} failed: {e}",
                            user.user_id.value()
                        );
                    }
                }
            }
        }

        let existing = self.wrapup_repo.find_existing(None, start, end).await?;
        if existing.is_none() {
            let cmd = crate::wrapup::commands::RequestWrapUpCommand {
                user_id: None,
                start_date: start,
                end_date: end,
            };
            if let Err(e) = crate::wrapup::generate::execute(
                self.wrapup_repo.clone(),
                self.event_publisher.clone(),
                cmd,
            )
            .await
            {
                tracing::warn!("auto-generate global wrapup failed: {e}");
            }
        }

        Ok(())
    }
}

pub struct WrapUpCleanupJob {
    wrapup_repo: Arc<dyn WrapUpRepository>,
}

impl WrapUpCleanupJob {
    pub fn new(wrapup_repo: Arc<dyn WrapUpRepository>) -> Self {
        Self { wrapup_repo }
    }
}

#[async_trait]
impl PeriodicJob for WrapUpCleanupJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86400)
    }

    async fn run(&self) -> Result<(), DomainError> {
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
        let n = self.wrapup_repo.delete_failed_older_than(cutoff).await?;
        if n > 0 {
            tracing::info!("wrapup cleanup: removed {n} failed records");
        }
        Ok(())
    }
}
