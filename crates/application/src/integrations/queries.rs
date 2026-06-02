use uuid::Uuid;

pub struct GetWatchQueueQuery {
    pub user_id: Uuid,
}

pub struct GetWebhookTokensQuery {
    pub user_id: Uuid,
}
