use domain::{
    errors::DomainError,
    models::collections::{PageParams, Paginated},
    models::{MovieFilter, MovieSummary},
};

use crate::{context::AppContext, queries::GetMoviesQuery};

pub async fn execute(ctx: &AppContext, query: GetMoviesQuery) -> Result<Paginated<MovieSummary>, DomainError> {
    let page = PageParams::new(query.limit, query.offset)?;
    let filter = MovieFilter {
        search: query.search,
        genre: query.genre,
        language: query.language,
    };
    ctx.movie_repository.list_movies(&page, &filter).await
}
