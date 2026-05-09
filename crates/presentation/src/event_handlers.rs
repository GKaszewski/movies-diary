use std::time::Duration;

use application::{commands::SyncPosterCommand, context::AppContext, use_cases::sync_poster};
use async_trait::async_trait;
use domain::ports::EventHandler;
use domain::{errors::DomainError, events::DomainEvent};

pub struct PosterSyncHandler {
    ctx: AppContext,
    max_retries: u32,
}

impl PosterSyncHandler {
    pub fn new(ctx: AppContext, max_retries: u32) -> Self {
        Self { ctx, max_retries }
    }
}

#[async_trait]
impl EventHandler for PosterSyncHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (movie_id, external_metadata_id) = match event {
            DomainEvent::MovieDiscovered {
                movie_id,
                external_metadata_id,
            } => (movie_id.value(), external_metadata_id.value().to_owned()),
            _ => return Ok(()),
        };

        let mut last_err: Option<DomainError> = None;
        for attempt in 0..=self.max_retries {
            let cmd = SyncPosterCommand {
                movie_id,
                external_metadata_id: external_metadata_id.clone(),
            };
            match sync_poster::execute(&self.ctx, cmd).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < self.max_retries {
                        let delay = Duration::from_secs(2u64.pow(attempt));
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = self.max_retries + 1,
                            delay_secs = delay.as_secs(),
                            "poster sync failed, retrying: {e}"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    last_err = Some(e);
                }
            }
        }

        let err = last_err.expect("loop runs at least once and always sets last_err on Err");
        tracing::error!(
            attempts = self.max_retries + 1,
            "poster sync failed after all attempts: {err}"
        );
        Err(err)
    }
}
