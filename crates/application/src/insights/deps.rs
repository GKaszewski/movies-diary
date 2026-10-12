use std::sync::Arc;

use domain::ports::InsightsQuery;

pub struct GetInsightsDeps {
    pub insights: Arc<dyn InsightsQuery>,
}
