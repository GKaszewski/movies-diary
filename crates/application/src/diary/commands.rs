use chrono::NaiveDateTime;
use domain::value_objects::WatchMedium;
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
    pub watch_medium: Option<WatchMedium>,
}

pub struct DeleteReviewCommand {
    pub review_id: Uuid,
    pub requesting_user_id: Uuid,
}

pub struct EditReviewCommand {
    pub review_id: Uuid,
    pub requesting_user_id: Uuid,
    pub rating: Option<u8>,
    pub comment: Option<Option<String>>,
    pub watched_at: Option<NaiveDateTime>,
    pub watch_medium: Option<Option<WatchMedium>>,
}

#[derive(Clone)]
pub struct SyncPosterCommand {
    pub movie_id: Uuid,
}
