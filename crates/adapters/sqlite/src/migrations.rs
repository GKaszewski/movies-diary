use std::time::Duration;

use domain::errors::DomainError;
use sqlx::SqlitePool;

const RETRY_BACKOFFS: [Duration; 5] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(1),
];

pub(crate) async fn run(pool: &SqlitePool) -> Result<(), DomainError> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    let mut backoffs = RETRY_BACKOFFS.iter();
    loop {
        match migrator.run(pool).await {
            Ok(()) => return Ok(()),
            Err(e) => match backoffs.next() {
                Some(delay) => {
                    tracing::warn!("migration failed, retrying (concurrent migrator?): {e}");
                    tokio::time::sleep(*delay).await;
                }
                None => return Err(DomainError::InfrastructureError(e.to_string())),
            },
        }
    }
}
