use std::sync::Arc;

use domain::ports::{WrapUpRepository, WrapUpStatsQuery};

pub struct HandleWrapUpRequestedDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
    pub wrapup_stats: Arc<dyn WrapUpStatsQuery>,
}

pub struct GetReadyReportDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}

pub struct DeleteWrapUpDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}

pub struct GenerateWrapUpDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}

pub struct GetWrapUpDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}

pub struct ListWrapUpsDeps {
    pub wrapup_repo: Arc<dyn WrapUpRepository>,
}
