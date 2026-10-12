mod federated_profile;
mod follow_repository;
mod social;
mod watchlist;

pub mod ap_content;
pub mod remote_goals;

pub use ap_content::PostgresApContentQuery;
pub use remote_goals::PostgresRemoteGoalRepository;

use sqlx::PgPool;

pub struct PostgresSocialRepository {
    pub(crate) pool: PgPool,
    pub(crate) instance: domain::value_objects::InstanceIdentity,
}

impl PostgresSocialRepository {
    pub fn new(pool: PgPool, instance: domain::value_objects::InstanceIdentity) -> Self {
        Self { pool, instance }
    }
}

pub fn create_federated_profile_query(
    pool: PgPool,
    instance: domain::value_objects::InstanceIdentity,
) -> std::sync::Arc<dyn domain::ports::FederatedProfileQuery> {
    std::sync::Arc::new(PostgresSocialRepository::new(pool, instance))
}
