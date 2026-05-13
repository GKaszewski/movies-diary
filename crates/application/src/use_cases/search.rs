use crate::context::AppContext;
use domain::{
    errors::DomainError,
    models::{SearchQuery, SearchResults},
};

pub async fn execute(ctx: &AppContext, query: SearchQuery) -> Result<SearchResults, DomainError> {
    ctx.search_port.search(&query).await
}
