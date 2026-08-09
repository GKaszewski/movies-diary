use std::sync::Arc;

use domain::ports::SearchPort;

pub struct SearchDeps {
    pub search_port: Arc<dyn SearchPort>,
}
