use std::sync::Arc;

use domain::ports::{BlockQuery, EventPublisher, FollowGraphQuery, SocialCommand};

pub struct SocialCommandDeps {
    pub social_command: Arc<dyn SocialCommand>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

pub struct SocialQueryDeps {
    pub follow_graph: Arc<dyn FollowGraphQuery>,
    pub block_query: Arc<dyn BlockQuery>,
}
