use std::sync::Arc;

use domain::ports::{
    DiaryQuery, EventPublisher, FederationAdminQuery, ObjectStorage, SocialQuery, StatsRepository,
    UserRepository,
};

pub struct GetProfileDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn SocialQuery>,
}

pub struct GetUsersListDeps {
    pub user: Arc<dyn UserRepository>,
    pub federation_admin: Arc<dyn FederationAdminQuery>,
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
