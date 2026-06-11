use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use domain::{
    errors::DomainError,
    events::DomainEvent,
    ports::{EventHandler, PersonCommand, PersonEnrichmentClient, PersonQuery},
};

const STALENESS_DAYS: i64 = 90;

pub struct PersonEnrichmentHandler {
    enrichment_client: Arc<dyn PersonEnrichmentClient>,
    person_query: Arc<dyn PersonQuery>,
    person_command: Arc<dyn PersonCommand>,
}

impl PersonEnrichmentHandler {
    pub fn new(
        enrichment_client: Arc<dyn PersonEnrichmentClient>,
        person_query: Arc<dyn PersonQuery>,
        person_command: Arc<dyn PersonCommand>,
    ) -> Self {
        Self {
            enrichment_client,
            person_query,
            person_command,
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

        if let Some(person) = self.person_query.get_by_id(&person_id).await? {
            if let Some(at) = person.enriched_at() {
                if (Utc::now() - at).num_days() < STALENESS_DAYS {
                    tracing::debug!(person_id = %person_id.value(), "person enrichment still fresh");
                    return Ok(());
                }
            }
        }

        tracing::info!(person_id = %person_id.value(), "enriching person from TMDb");
        let data = self
            .enrichment_client
            .fetch_details(&external_person_id)
            .await?;
        self.person_command
            .update_enrichment(&person_id, &data)
            .await
    }
}
