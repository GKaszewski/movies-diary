use uuid::Uuid;

pub struct GetWatchlistQuery {
    pub user_id: Uuid,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub struct IsOnWatchlistQuery {
    pub user_id: Uuid,
    pub movie_id: Uuid,
}
