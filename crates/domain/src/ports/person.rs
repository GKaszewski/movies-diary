use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{ExternalPersonId, Person, PersonCredits, PersonEnrichmentData, PersonId},
};

#[async_trait]
pub trait PersonEnrichmentClient: Send + Sync {
    async fn fetch_details(&self, external_id: &str) -> Result<PersonEnrichmentData, DomainError>;
}

/// Write port — mutates the persons table. No reads.
#[async_trait]
pub trait PersonCommand: Send + Sync {
    /// Upsert a batch of persons. Uses INSERT OR REPLACE (SQLite) / ON CONFLICT DO UPDATE (Postgres).
    async fn upsert_batch(&self, persons: &[Person]) -> Result<(), DomainError>;
    /// Insert a batch of missing persons from movie_cast/movie_crew into the persons table.
    /// Returns (inserted_count, has_more).
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

/// Read port — queries persons and credits. No mutations.
#[async_trait]
pub trait PersonQuery: Send + Sync {
    async fn get_by_id(&self, id: &PersonId) -> Result<Option<Person>, DomainError>;
    async fn get_by_external_id(
        &self,
        id: &ExternalPersonId,
    ) -> Result<Option<Person>, DomainError>;
    /// Returns the person's full cast and crew credit history across all indexed movies.
    async fn get_credits(&self, id: &PersonId) -> Result<PersonCredits, DomainError>;
    /// Returns persons who have no remaining entries in movie_cast or movie_crew.
    /// Called after movie deletion to find index entries that can be pruned.
    async fn list_orphaned_persons(&self) -> Result<Vec<PersonId>, DomainError>;
    async fn list_page(&self, limit: u32, offset: u32) -> Result<Vec<Person>, DomainError>;
}
