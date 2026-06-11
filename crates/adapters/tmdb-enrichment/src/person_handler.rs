use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, PersonCommand, PersonEnrichmentClient, PersonQuery},
};

use application::person::deps::EnrichPersonDeps;

pub struct PersonEnrichmentHandler {
    deps: EnrichPersonDeps,
}

impl PersonEnrichmentHandler {
    pub fn new(
        person_query: Arc<dyn PersonQuery>,
        person_enrichment: Option<Arc<dyn PersonEnrichmentClient>>,
        person_command: Arc<dyn PersonCommand>,
    ) -> Self {
        Self {
            deps: EnrichPersonDeps {
                person_query,
                person_enrichment,
                person_command,
            },
        }
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

        application::person::enrich::execute(&self.deps, person_id, external_person_id.value())
            .await
    }
}
