use domain::{
    errors::DomainError,
    models::{
        FeedEntry, Movie, MovieProfile, MovieStats,
        collections::{PageParams, Paginated},
    },
    value_objects::MovieId,
};

use crate::{context::AppContext, queries::GetMovieSocialPageQuery};

pub struct MovieSocialPageResult {
    pub movie: Movie,
    pub stats: MovieStats,
    pub reviews: Paginated<FeedEntry>,
    pub profile: Option<MovieProfile>,
}

pub async fn execute(
    ctx: &AppContext,
    query: GetMovieSocialPageQuery,
) -> Result<MovieSocialPageResult, DomainError> {
    let movie_id = MovieId::from_uuid(query.movie_id);
    let page = PageParams::new(Some(query.limit), Some(query.offset))?;

    let movie = ctx
        .movie_repository
        .get_movie_by_id(&movie_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("Movie {}", query.movie_id)))?;

    let (stats, reviews, profile) = tokio::try_join!(
        ctx.diary_repository.get_movie_stats(&movie_id),
        ctx.diary_repository.get_movie_social_feed(&movie_id, &page),
        ctx.movie_profile_repository.get_by_movie_id(&movie_id),
    )?;

    Ok(MovieSocialPageResult {
        movie,
        stats,
        reviews,
        profile,
    })
}
