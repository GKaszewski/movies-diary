use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{EntityType, IndexableDocument, SearchQuery, SearchResults},
};

/// Read port — executes search queries. No mutations.
#[async_trait]
pub trait SearchPort: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, DomainError>;
}

/// Write port — manages the search index. No reads.
#[async_trait]
pub trait SearchCommand: Send + Sync {
    /// Add or replace a document in the search index.
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError>;
    /// Remove a document from the search index by entity type and internal ID string.
    async fn remove(&self, entity_type: EntityType, id: &str) -> Result<(), DomainError>;
}
