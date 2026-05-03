use domain::errors::DomainError;
use sqlx::SqlitePool;

pub(crate) async fn run(pool: &SqlitePool) -> Result<(), DomainError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))
}
