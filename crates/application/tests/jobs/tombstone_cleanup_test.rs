use std::sync::{Arc, Mutex};

use application::jobs::{TOMBSTONE_RETENTION_DAYS, TombstoneCleanupJob};
use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{AccountPurger, PeriodicJob},
    value_objects::UserId,
};

#[derive(Default)]
struct Purger {
    cutoffs: Mutex<Vec<chrono::NaiveDateTime>>,
}

#[async_trait]
impl AccountPurger for Purger {
    async fn is_purged(&self, _: &UserId) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn owned_image_keys(&self, _: &UserId) -> Result<Vec<String>, DomainError> {
        unimplemented!()
    }
    async fn purge(&self, _: &UserId) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn forget_images(&self, _: &UserId) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn purge_expired_tombstones(
        &self,
        cutoff: chrono::NaiveDateTime,
    ) -> Result<u64, DomainError> {
        self.cutoffs.lock().unwrap().push(cutoff);
        Ok(0)
    }
}

#[tokio::test]
async fn removes_tombstones_purged_before_the_retention_window() {
    assert_eq!(TOMBSTONE_RETENTION_DAYS, 7);
    let purger = Arc::new(Purger::default());

    TombstoneCleanupJob::new(purger.clone())
        .run()
        .await
        .unwrap();

    let cutoffs = purger.cutoffs.lock().unwrap();
    let expected = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
    assert_eq!(cutoffs.len(), 1);
    assert!((cutoffs[0] - expected).num_seconds().abs() < 5);
}

#[test]
fn runs_daily() {
    let job = TombstoneCleanupJob::new(Arc::new(Purger::default()));
    assert_eq!(job.interval(), std::time::Duration::from_secs(86_400));
}
