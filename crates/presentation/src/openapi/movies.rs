use api_types::{
    CastMemberDto, CrewMemberDto, DirectorStatDto, GenreDto, KeywordDto, MonthActivityDto,
    MonthlyRatingDto, MovieDetailResponse, MovieDto, MovieProfileResponse, MovieStatsDto,
    MoviesResponse, ReviewHistoryResponse, SocialFeedResponse, SocialReviewDto, UserTrendsDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::movies::list_movies,
        crate::handlers::movies::get_movie_detail,
        crate::handlers::movies::get_review_history,
        crate::handlers::movies::get_movie_profile,
        crate::handlers::movies::sync_poster,
    ),
    components(schemas(
        MoviesResponse,
        MovieDto,
        MovieDetailResponse,
        MovieStatsDto,
        MovieProfileResponse,
        GenreDto,
        KeywordDto,
        CastMemberDto,
        CrewMemberDto,
        ReviewHistoryResponse,
        SocialFeedResponse,
        SocialReviewDto,
        MonthActivityDto,
        MonthlyRatingDto,
        DirectorStatDto,
        UserTrendsDto,
    ))
)]
pub struct MoviesDoc;
