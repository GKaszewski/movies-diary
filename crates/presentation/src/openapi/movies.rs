use api_types::{
    DirectorStatDto, MonthActivityDto, MonthlyRatingDto, MovieDetailResponse, MovieDto,
    MovieStatsDto, ReviewHistoryResponse, SocialFeedResponse, SocialReviewDto, UserTrendsDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::get_movie_detail,
        crate::handlers::api::get_review_history,
        crate::handlers::api::sync_poster,
    ),
    components(schemas(
        MovieDto,
        MovieDetailResponse,
        MovieStatsDto,
        ReviewHistoryResponse,
        SocialFeedResponse,
        SocialReviewDto,
        MonthActivityDto,
        MonthlyRatingDto,
        DirectorStatDto,
        UserTrendsDto,
    )),
)]
pub struct MoviesDoc;
