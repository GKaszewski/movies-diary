use std::sync::Arc;

use domain::ports::{
    DiaryExporter, DiaryQuery, FollowGraphQuery, MovieProfileRepository, MovieQuery,
    ReviewRepository, UserRepository,
};

use crate::config::AppSettings;

pub struct DeleteReviewDeps {
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryQuery>,
}

pub struct EditReviewDeps {
    pub review: Arc<dyn ReviewRepository>,
}

pub struct GetMovieSocialPageDeps {
    pub movie_query: Arc<dyn MovieQuery>,
    pub diary: Arc<dyn DiaryQuery>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
}

pub struct GetActivityFeedDeps {
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn FollowGraphQuery>,
    pub config: AppSettings,
}

pub struct GetUserFeedDeps {
    pub user: Arc<dyn UserRepository>,
    pub diary: Arc<dyn DiaryQuery>,
}

pub struct GetDiaryDeps {
    pub diary: Arc<dyn DiaryQuery>,
}

pub struct GetReviewHistoryDeps {
    pub diary: Arc<dyn DiaryQuery>,
}

pub struct ExportDiaryDeps {
    pub diary: Arc<dyn DiaryQuery>,
    pub diary_exporter: Arc<dyn DiaryExporter>,
}
