use api_types::{
    ActivityFeedResponse, DiaryEntryDto, DiaryResponse, FeedEntryDto, LogReviewRequest, ReviewDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::diary::get_diary,
        crate::api::diary::post_review,
        crate::api::diary::delete_review,
        crate::api::diary::export_diary,
        crate::api::diary::get_activity_feed,
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
