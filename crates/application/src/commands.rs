use chrono::NaiveDateTime;
use uuid::Uuid;

pub struct LogReviewCommand {
    pub external_metadata_id: Option<String>,

    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,

    pub user_id: Uuid,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: NaiveDateTime,
}

pub struct SyncPosterCommand {
    pub movie_id: Uuid,
    pub external_metadata_id: String,
}

pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

pub struct RegisterCommand {
    pub email: String,
    pub password: String,
}
