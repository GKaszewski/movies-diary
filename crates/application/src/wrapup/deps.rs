use std::sync::Arc;

use domain::ports::{EventPublisher, WrapUpRepository, WrapUpStatsQuery};

pub struct HandleWrapUpRequestedDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
    pub wrapup_stats: Arc<dyn WrapUpStatsQuery>,
}
