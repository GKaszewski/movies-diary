use std::sync::Arc;
use domain::{
    errors::DomainError,
    models::{SearchQuery, SearchResults},
    ports::SearchPort,
};

pub async fn execute(
    search_port: Arc<dyn SearchPort>,
    query: SearchQuery,
) -> Result<SearchResults, DomainError> {
    search_port.search(&query).await
}

#[cfg(test)]
#[path = "tests/execute.rs"]
mod tests;
