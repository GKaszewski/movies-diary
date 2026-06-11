use async_trait::async_trait;
use domain::{errors::DomainError, events::DomainEvent, ports::EventHandler};

use application::context::AppContext;

pub struct PersonEnrichmentHandler {
    ctx: AppContext,
}

impl PersonEnrichmentHandler {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl EventHandler for PersonEnrichmentHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let (person_id, external_person_id) = match event {
            DomainEvent::PersonEnrichmentRequested {
                person_id,
                external_person_id,
            } => (person_id.clone(), external_person_id.clone()),
            _ => return Ok(()),
        };

        application::person::enrich::execute(&self.ctx, person_id, &external_person_id).await
    }
}
