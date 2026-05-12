use domain::{errors::DomainError, models::{SearchQuery, SearchResults}};
use crate::context::AppContext;

pub async fn execute(ctx: &AppContext, query: SearchQuery) -> Result<SearchResults, DomainError> {
    ctx.search_port.search(&query).await
}
