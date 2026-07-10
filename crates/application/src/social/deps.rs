use std::sync::Arc;

use domain::ports::{EventPublisher, SocialCommand, SocialQuery};

pub struct SocialCommandDeps {
    pub social_command: Arc<dyn SocialCommand>,
    pub social_query: Arc<dyn SocialQuery>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct SocialQueryDeps {
    pub social_query: Arc<dyn SocialQuery>,
}
