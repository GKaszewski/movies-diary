use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use domain::{
    errors::DomainError,
    ports::{MovieDeduplicator, MovieRepository, ObjectStorage, PeriodicJob},
};

use crate::movies::merge_duplicates::{MergeDuplicatesDeps, execute};

pub struct MovieDeduplicationJob {
    deps: MergeDuplicatesDeps,
}

impl MovieDeduplicationJob {
    pub fn new(
        movie: Arc<dyn MovieRepository>,
        deduplicator: Arc<dyn MovieDeduplicator>,
        object_storage: Arc<dyn ObjectStorage>,
    ) -> Self {
        Self {
            deps: MergeDuplicatesDeps {
                movie,
                deduplicator,
                object_storage,
            },
        }
    }
}

#[async_trait]
impl PeriodicJob for MovieDeduplicationJob {
    fn interval(&self) -> Duration {
        Duration::from_secs(86_400) // once per day
    }

    async fn run(&self) -> Result<(), DomainError> {
        let report = execute(&self.deps).await?;
        if report.pairs_found > 0 {
            tracing::info!(
                pairs_found = report.pairs_found,
                rows_repointed = report.rows_repointed,
                "movie dedup: merged duplicate records"
            );
        }
        Ok(())
    }
}
