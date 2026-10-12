use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{ExternalPersonId, Person, PersonCredits, PersonEnrichmentData, PersonId},
};

#[async_trait]
pub trait PersonEnrichmentClient: Send + Sync {
    async fn fetch_details(&self, external_id: &str) -> Result<PersonEnrichmentData, DomainError>;
}

#[async_trait]
pub trait PersonCommand: Send + Sync {
    async fn upsert_batch(&self, persons: &[Person]) -> Result<(), DomainError>;
    async fn backfill_from_credits_batch(
        &self,
        batch_size: u32,
    ) -> Result<(u64, bool), DomainError>;
    async fn update_enrichment(
        &self,
        id: &PersonId,
        data: &PersonEnrichmentData,
    ) -> Result<(), DomainError>;
}

#[async_trait]
pub trait PersonQuery: Send + Sync {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError>;
    async fn get_by_external_id(
        &self,
        id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError>;
    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError>;
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError>;
    async fn list_page(&self, limit: u32, offset: u32) -> Result<Vec<Person>, DomainError>;
}
