use std::sync::Arc;

use domain::{
    errors::DomainError,
    models::collections::{PageParams, Paginated},
    models::{MovieFilter, MovieSummary},
    ports::MovieRepository,
};

use crate::movies::queries::GetMoviesQuery;

pub async fn execute(
    movie: Arc<dyn MovieRepository>,
    query: GetMoviesQuery,
) -> Result<Paginated<MovieSummary>, DomainError> {
    let page = PageParams::new(query.limit, query.offset)?;
    let filter = MovieFilter {
        search: query.search,
        genre: query.genre,
        language: query.language,
    };
    movie.list_movies(&page, &filter).await
}

#[cfg(test)]
#[path = "tests/get_movies.rs"]
mod tests;
