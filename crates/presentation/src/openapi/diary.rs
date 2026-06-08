use api_types::{
    ActivityFeedResponse, DiaryEntryDto, DiaryResponse, FeedEntryDto, LogReviewRequest, ReviewDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::diary::get_diary,
        crate::handlers::diary::post_review,
        crate::handlers::diary::delete_review,
        crate::handlers::diary::export_diary,
        crate::handlers::diary::get_activity_feed,
    ),
    components(schemas(
        DiaryResponse,
        DiaryEntryDto,
        ReviewDto,
        LogReviewRequest,
        ActivityFeedResponse,
        FeedEntryDto,
    ))
)]
pub struct DiaryDoc;
