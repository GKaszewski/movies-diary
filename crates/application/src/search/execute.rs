use domain::{
    errors::DomainError,
    models::{SearchQuery, SearchResults},
};

use crate::search::deps::SearchDeps;

pub async fn execute(deps: &SearchDeps, query: SearchQuery) -> Result<SearchResults, DomainError> {
    deps.search_port.search(&query).await
}

#[cfg(test)]
#[path = "tests/execute.rs"]
mod tests;
