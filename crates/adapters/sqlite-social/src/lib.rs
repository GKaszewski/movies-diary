mod federated_profile;
mod follow_repository;
mod social;
mod watchlist;

pub mod ap_content;
pub mod remote_goals;

pub use ap_content::SqliteApContentQuery;
pub use remote_goals::SqliteRemoteGoalRepository;

use sqlx::SqlitePool;

pub struct SqliteSocialRepository {
    pub(crate) pool: SqlitePool,
    pub(crate) instance: domain::value_objects::InstanceIdentity,
}

impl SqliteSocialRepository {
    pub fn new(pool: SqlitePool, instance: domain::value_objects::InstanceIdentity) -> Self {
        Self { pool, instance }
    }
}

pub fn create_federated_profile_query(
    pool: SqlitePool,
    instance: domain::value_objects::InstanceIdentity,
) -> std::sync::Arc<dyn domain::ports::FederatedProfileQuery> {
    std::sync::Arc::new(SqliteSocialRepository::new(pool, instance))
}

#[cfg(test)]
#[path = "tests/follow_relation_tests.rs"]
mod follow_relation_tests;
