use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct RemoteWatchlistEntry {
    pub ap_id: String,
    pub actor_url: String,
    pub movie_title: String,
    pub release_year: u16,
    pub external_metadata_id: Option<String>,
    pub poster_url: Option<String>,
    pub added_at: DateTime<Utc>,
}
