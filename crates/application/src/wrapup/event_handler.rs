use std::sync::Arc;

use async_trait::async_trait;
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::ports::EventHandler;
use tokio::sync::Semaphore;

use crate::context::AppContext;

pub struct WrapUpEventHandler {
    ctx: AppContext,
    semaphore: Arc<Semaphore>,
}

impl WrapUpEventHandler {
    pub fn new(ctx: AppContext) -> Self {
        let max = ctx.config.wrapup.max_concurrent_renders;
        Self {
            ctx,
            semaphore: Arc::new(Semaphore::new(max)),
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
                    &self.ctx,
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
