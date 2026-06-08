use uuid::Uuid;

pub struct CreateGoalCommand {
    pub user_id: Uuid,
    pub year: u16,
    pub target_count: u32,
}

pub struct UpdateGoalCommand {
    pub user_id: Uuid,
    pub year: u16,
    pub target_count: u32,
}

pub struct DeleteGoalCommand {
    pub user_id: Uuid,
    pub year: u16,
}
