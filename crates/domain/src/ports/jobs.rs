use async_trait::async_trait;

use crate::errors::DomainError;

#[async_trait]
pub trait PeriodicJob: Send + Sync {
    fn interval(&self) -> std::time::Duration;
    async fn run(&self) -> Result<(), DomainError>;
}
