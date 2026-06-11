use std::sync::Arc;

use domain::ports::{EventPublisher, PersonCommand, PersonEnrichmentClient, PersonQuery};

pub struct GetPersonDeps {
    pub person_query: Arc<dyn PersonQuery>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct EnrichPersonDeps {
    pub person_query: Arc<dyn PersonQuery>,
    pub person_enrichment: Option<Arc<dyn PersonEnrichmentClient>>,
    pub person_command: Arc<dyn PersonCommand>,
}
