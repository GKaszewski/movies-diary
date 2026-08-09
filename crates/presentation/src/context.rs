use std::sync::Arc;

use application::config::AppConfig;

#[derive(Clone)]
pub struct AppContext {
    pub deps: Arc<application::Deps>,
    pub services: application::Services,
    pub config: AppConfig,
    pub instance: domain::value_objects::InstanceIdentity,
    #[cfg(feature = "federation")]
    pub ap_document: Arc<dyn domain::ports::ApDocumentPort>,
    #[cfg(feature = "federation")]
    pub ap_blocklist: Arc<dyn domain::ports::InstanceBlocklistPort>,
}
