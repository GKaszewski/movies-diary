use async_trait::async_trait;

use crate::{
    errors::DomainError,
    models::{EntityType, IndexableDocument, SearchQuery, SearchResults},
};

#[async_trait]
pub trait SearchPort: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults, DomainError>;
}

#[async_trait]
pub trait SearchCommand: Send + Sync {
    async fn index(&self, doc: IndexableDocument) -> Result<(), DomainError>;
    async fn remove(&self, entity_type: EntityType, id: &str) -> Result<(), DomainError>;
}
