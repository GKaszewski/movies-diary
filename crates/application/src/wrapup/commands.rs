use chrono::NaiveDate;
use uuid::Uuid;

pub struct RequestWrapUpCommand {
    pub user_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}
