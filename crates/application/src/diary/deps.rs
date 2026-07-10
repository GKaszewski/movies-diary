use std::sync::Arc;

use domain::ports::{
    DiaryQuery, EventPublisher, MovieCommand, MovieProfileRepository, MovieQuery, ReviewRepository,
    SocialQueryPort,
};

use crate::config::AppConfig;

pub struct DeleteReviewDeps {
    pub review: Arc<dyn ReviewRepository>,
    pub diary: Arc<dyn DiaryQuery>,
    pub movie_command: Arc<dyn MovieCommand>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct EditReviewDeps {
    pub review: Arc<dyn ReviewRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct GetMovieSocialPageDeps {
    pub movie_query: Arc<dyn MovieQuery>,
    pub diary: Arc<dyn DiaryQuery>,
    pub movie_profile: Arc<dyn MovieProfileRepository>,
}

pub struct GetActivityFeedDeps {
    pub diary: Arc<dyn DiaryQuery>,
    pub social_query: Arc<dyn SocialQueryPort>,
    pub config: AppConfig,
}
