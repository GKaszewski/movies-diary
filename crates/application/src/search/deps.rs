use std::sync::Arc;

use domain::ports::{EventPublisher, SearchPort};

pub struct SearchDeps {
    pub search_port: Arc<dyn SearchPort>,
}

pub struct RequestReindexDeps {
    pub event_publisher: Arc<dyn EventPublisher>,
}
