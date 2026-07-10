use uuid::Uuid;

pub enum SocialQry {
    GetFollowing { user_id: Uuid },
    GetFollowers { user_id: Uuid },
    GetPending { user_id: Uuid },
    GetBlocked { user_id: Uuid },
}
