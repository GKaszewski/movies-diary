use domain::{
    errors::DomainError,
    models::{
        FeedEntry, Movie, MovieProfile, MovieStats,
        collections::{PageParams, Paginated},
    },
    value_objects::MovieId,
};

use crate::diary::{deps::GetMovieSocialPageDeps, queries::GetMovieSocialPageQuery};

pub struct MovieSocialPageResult {
    pub movie: Movie,
    pub stats: MovieStats,
    pub reviews: Paginated<FeedEntry>,
    pub profile: Option<MovieProfile>,
}

pub async fn execute(
    deps: &GetMovieSocialPageDeps,
    query: GetMovieSocialPageQuery,
) -> Result<MovieSocialPageResult, DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);
    let page = PageParams::new(Some(query.limit), Some(query.offset))?;

    let movie = deps
        .movie
        .get_movie_by_id(&movie_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", query.movie_id)))?;

    let (stats, reviews, profile) = tokio::try_join!(
        deps.diary.get_movie_stats(&movie_id),
        deps.diary.get_movie_social_feed(&movie_id, &page),
        deps.movie_profile.get_by_movie_id(&movie_id),
    )?;

    Ok(MovieSocialPageResult {
        movie,
        stats,
        reviews,
        profile,
    })
}

#[cfg(test)]
#[path = "tests/get_movie_social_page.rs"]
mod tests;
