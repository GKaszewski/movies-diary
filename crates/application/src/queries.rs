use domain::models::SortDirection;
use uuid::Uuid;

pub struct GetDiaryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<SortDirection>,
    pub movie_id: Option<Uuid>,
}

pub struct GetReviewHistoryQuery {
    pub movie_id: Uuid,
}

pub struct GetActivityFeedQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub struct GetUsersQuery;

pub struct GetUserProfileQuery {
    pub user_id: Uuid,
    pub view: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
