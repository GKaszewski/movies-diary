use chrono::NaiveDateTime;
use uuid::Uuid;

pub struct MovieInput {
    pub movie_id: Option<Uuid>,
    pub external_metadata_id: Option<String>,
    pub manual_title: Option<String>,
    pub manual_release_year: Option<u16>,
    pub manual_director: Option<String>,
}

pub struct LogReviewCommand {
    pub user_id: Uuid,
    pub input: MovieInput,
    pub rating: u8,
    pub comment: Option<String>,
    pub watched_at: NaiveDateTime,
}

pub struct DeleteReviewCommand {
    pub review_id: Uuid,
    pub requesting_user_id: Uuid,
}

#[derive(Clone)]
pub struct SyncPosterCommand {
    pub movie_id: Uuid,
}
