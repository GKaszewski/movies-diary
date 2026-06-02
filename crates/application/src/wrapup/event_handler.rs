use async_trait::async_trait;
use domain::errors::DomainError;
use domain::events::DomainEvent;
use domain::ports::EventHandler;

use crate::context::AppContext;

pub struct WrapUpEventHandler {
    ctx: AppContext,
}

impl WrapUpEventHandler {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
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
