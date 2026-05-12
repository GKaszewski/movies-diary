use api_types::{
    CastMemberDto, CrewMemberDto, DirectorStatDto, GenreDto, KeywordDto, MonthActivityDto,
    MonthlyRatingDto, MovieDetailResponse, MovieDto, MovieProfileResponse, MovieStatsDto,
    MoviesResponse, ReviewHistoryResponse, SocialFeedResponse, SocialReviewDto, UserTrendsDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::list_movies,
        crate::handlers::api::get_movie_detail,
        crate::handlers::api::get_review_history,
        crate::handlers::api::get_movie_profile,
        crate::handlers::api::sync_poster,
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
