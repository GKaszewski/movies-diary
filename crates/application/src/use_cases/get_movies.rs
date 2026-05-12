use domain::{
    errors::DomainError,
    models::collections::{PageParams, Paginated},
    models::Movie,
};

use crate::{context::AppContext, queries::GetMoviesQuery};

pub async fn execute(ctx: &AppContext, query: GetMoviesQuery) -> Result<Paginated<Movie>, DomainError> {
    let page = PageParams::new(query.limit, query.offset)?;
    ctx.movie_repository
        .list_movies(&page, query.search.as_deref())
        .await
}
