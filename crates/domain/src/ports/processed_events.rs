use async_trait::async_trait;

use crate::{errors::DomainError, events::EventId};

pub const HANDLER_LEASE_SECONDS: i64 = 1800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimToken(String);

impl ClaimToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Begin {
    Run(ClaimToken),
    AlreadyDone,
    InProgressElsewhere { retry_after: std::time::Duration },
}

#[async_trait]
pub trait ProcessedEventLedger: Send + Sync {
    async fn begin(&self, id: &EventId, handler: &str) -> Result<Begin, DomainError>;

    async fn complete(
        &self,
        id: &EventId,
        handler: &str,
        token: &ClaimToken,
    ) -> Result<(), DomainError>;

    async fn release(
        &self,
        id: &EventId,
        handler: &str,
        token: &ClaimToken,
    ) -> Result<(), DomainError>;

    async fn delete_older_than(&self, cutoff: chrono::NaiveDateTime) -> Result<u64, DomainError>;
}
