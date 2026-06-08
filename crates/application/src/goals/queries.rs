use uuid::Uuid;

pub struct GetGoalQuery {
    pub user_id: Uuid,
    pub year: u16,
}

pub struct ListGoalsQuery {
    pub user_id: Uuid,
}
