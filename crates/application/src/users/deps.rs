use std::sync::Arc;

use domain::ports::{
    DiaryQuery, EventPublisher, ObjectStorage, SocialQuery, SocialQueryPort, StatsRepository,
    UserRepository,
};

pub struct GetProfileDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn SocialQuery>,
}

pub struct GetUsersListDeps {
    pub user: Arc<dyn UserRepository>,
    pub social_query_legacy: Arc<dyn SocialQueryPort>,
}

pub struct UpdateProfileDeps {
    pub user: Arc<dyn UserRepository>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct DeleteAccountDeps {
    pub user: Arc<dyn UserRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}
