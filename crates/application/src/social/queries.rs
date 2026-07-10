use uuid::Uuid;

pub struct GetFollowingQuery {
    pub user_id: Uuid,
}

pub struct GetFollowersQuery {
    pub user_id: Uuid,
}

pub struct GetPendingFollowersQuery {
    pub user_id: Uuid,
}

pub struct GetBlockedQuery {
    pub user_id: Uuid,
}
