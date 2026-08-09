use std::sync::Arc;

use domain::ports::{
    DiaryQuery, EventPublisher, FederatedProfileQuery, FederationAdminQuery, FollowGraphQuery,
    ObjectStorage, StatsRepository, UserProfileFieldsRepository, UserRepository,
    UserSettingsRepository,
};
use domain::value_objects::InstanceIdentity;

/// The local half of the former `GetProfileDeps` split (ADR-0004). `user` and
/// `instance` build the always-populated `ProfileIdentity`; `stats`/`diary`/
/// `social_query` are unchanged from before the split.
pub struct GetLocalProfileDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn FollowGraphQuery>,
    pub user: Arc<dyn UserRepository>,
    pub instance: InstanceIdentity,
}

/// The federated half of the former `GetProfileDeps` split. No `user`/`instance` —
/// the federated handler builds identity from the resolved remote actor and never
/// reads a local user row for this path.
pub struct GetFederatedProfileStatsDeps {
    pub stats: Arc<dyn StatsRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn FollowGraphQuery>,
}

/// Backs page-chrome data (email/role badge, pending-follow badge). See the error
/// policy note on `get_page_viewer::execute`: this deliberately propagates errors
/// rather than degrading internally — the caller decides chrome degrades.
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

pub struct DeleteAccountDeps {
    pub user: Arc<dyn UserRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct GetCurrentProfileDeps {
    pub user: Arc<dyn UserRepository>,
}

pub struct UpdateProfileFieldsDeps {
    pub profile_fields: Arc<dyn UserProfileFieldsRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
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

/// `Option` mirrors `Repositories::federated_profile` — genuine optional
/// configuration (federation on/off), not a container-shape workaround. Same
/// exemption as `Services::person_enrichment`.
pub struct GetFederatedProfileDeps {
    pub federated_profile: Option<Arc<dyn FederatedProfileQuery>>,
}
