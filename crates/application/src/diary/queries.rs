use domain::models::ReviewSortBy;
use uuid::Uuid;

pub struct GetDiaryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<ReviewSortBy>,
    pub movie_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}

pub struct GetReviewHistoryQuery {
    pub movie_id: Uuid,
}

pub struct GetActivityFeedQuery {
    pub limit: u32,
    pub offset: u32,
    pub sort_by: domain::models::FeedSortBy,
    pub search: Option<String>,
    pub viewer_user_id: Option<Uuid>,
    pub filter_following: bool,
}

pub struct ExportQuery {
    pub user_id: Uuid,
    pub format: domain::models::ExportFormat,
}

pub struct GetMovieSocialPageQuery {
    pub movie_id: uuid::Uuid,
    pub limit: u32,
    pub offset: u32,
}
