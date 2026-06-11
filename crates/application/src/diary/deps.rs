use std::sync::Arc;

use domain::ports::{
    DiaryRepository, EventPublisher, MovieProfileRepository, MovieRepository, ReviewRepository,
    SocialQueryPort,
};

use crate::config::AppConfig;

pub struct DeleteReviewDeps {
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub movie: Arc<dyn MovieRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct GetMovieSocialPageDeps {
    pub movie: Arc<dyn MovieRepository>,
    pub diary: Arc<dyn DiaryRepository>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
}

pub struct GetActivityFeedDeps {
    pub diary: Arc<dyn DiaryRepository>,
    pub social_query: Arc<dyn SocialQueryPort>,
    pub config: AppConfig,
}
