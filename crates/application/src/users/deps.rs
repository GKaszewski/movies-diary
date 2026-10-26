use std::sync::Arc;

use domain::ports::{
    DiaryQuery, EventPublisher, FederatedProfileQuery, FederationAdminQuery, FollowGraphQuery,
    ObjectStorage, PasswordHasher, StatsRepository, UserProfileFieldsRepository, UserRepository,
    UserSettingsRepository,
};
use domain::value_objects::InstanceIdentity;

pub struct GetLocalProfileDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn FollowGraphQuery>,
    pub user: Arc<dyn UserRepository>,
    pub instance: InstanceIdentity,
}

pub struct GetFederatedProfileStatsDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn FollowGraphQuery>,
}

pub struct GetPageViewerDeps {
    pub user: Arc<dyn UserRepository>,
    pub follow_graph: Arc<dyn FollowGraphQuery>,
}

pub struct ResolveUsernameDeps {
    pub user: Arc<dyn UserRepository>,
}

pub struct GetProfileSettingsDeps {
    pub user: Arc<dyn UserRepository>,
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub instance: InstanceIdentity,
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

pub struct RequestAccountDeletionDeps {
    pub user: Arc<dyn UserRepository>,
    pub password_hasher: Arc<dyn PasswordHasher>,
}

pub struct EnsureActiveDeps {
    pub user: Arc<dyn UserRepository>,
}

pub struct GetCurrentProfileDeps {
    pub user: Arc<dyn UserRepository>,
}

pub struct UpdateProfileFieldsDeps {
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
}

pub struct GetSettingsDeps {
    pub user_settings: Arc<dyn UserSettingsRepository>,
}

pub struct UpdateSettingsDeps {
    pub user_settings: Arc<dyn UserSettingsRepository>,
}

pub struct AuthorizeAdminDeps {
    pub user: Arc<dyn UserRepository>,
}

pub struct GetFederatedProfileDeps {
    pub federated_profile: Option<Arc<dyn FederatedProfileQuery>>,
}
