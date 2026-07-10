use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PaginationQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedResponse<T: std::fmt::Debug + Clone> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

pub type MoviesResponse = PaginatedResponse<crate::movies::MovieDto>;
pub type SocialFeedResponse = PaginatedResponse<crate::movies::SocialReviewDto>;
pub type DiaryResponse = PaginatedResponse<crate::diary::DiaryEntryDto>;
pub type ActivityFeedResponse = PaginatedResponse<crate::diary::FeedEntryDto>;
pub type WatchlistResponse = PaginatedResponse<crate::watchlist::WatchlistEntryDto>;
pub type PaginatedMovieHits = PaginatedResponse<crate::search::MovieSearchHitDto>;
pub type PaginatedPersonHits = PaginatedResponse<crate::search::PersonSearchHitDto>;
