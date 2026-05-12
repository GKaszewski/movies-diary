use api_types::{ActivityFeedResponse, DiaryEntryDto, DiaryResponse, FeedEntryDto, LogReviewRequest, ReviewDto};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::api::get_diary,
        crate::handlers::api::post_review,
        crate::handlers::api::delete_review,
        crate::handlers::api::export_diary,
        crate::handlers::api::get_activity_feed,
    ),
    components(schemas(
        DiaryResponse,
        DiaryEntryDto,
        ReviewDto,
        LogReviewRequest,
        ActivityFeedResponse,
        FeedEntryDto,
    )),
)]
pub struct DiaryDoc;
