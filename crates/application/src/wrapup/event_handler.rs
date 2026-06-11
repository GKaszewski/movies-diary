use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::ports::{EventHandler, EventPublisher, WrapUpRepository, WrapUpStatsQuery};
use tokio::sync::Semaphore;

use super::deps::HandleWrapUpRequestedDeps;

pub struct WrapUpEventHandler {
    deps: HandleWrapUpRequestedDeps,
    semaphore: Arc<Semaphore>,
}

impl WrapUpEventHandler {
    pub fn new(
        wrapup_repo: Arc<dyn WrapUpRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        wrapup_stats: Arc<dyn WrapUpStatsQuery>,
    ) -> Self {
        Self {
            deps: HandleWrapUpRequestedDeps {
                wrapup_repo,
                event_publisher,
                wrapup_stats,
            },
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }
}

#[async_trait]
impl EventHandler for WrapUpEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        match event {
            DomainEvent::WrapUpRequested {
                wrapup_id,
                user_id,
                start_date,
                end_date,
            } => {
                let _permit = self.semaphore.acquire().await.map_err(|_| {
                    DomainError::InfrastructureError("render semaphore closed".into())
                })?;
                super::handle_requested::execute(
                    &self.deps,
                    wrapup_id.clone(),
                    user_id.as_ref().map(|u| u.value()),
                    *start_date,
                    *end_date,
                )
                .await
            }
            _ => Ok(()),
        }
    }
}
