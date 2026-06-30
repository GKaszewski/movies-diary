mod activity;
mod actor;
pub mod ap_content;
mod blocklist;
mod follow;
pub mod remote_goals;
mod review;
mod social;
mod watchlist;

pub use ap_content::PostgresApContentQuery;
pub use remote_goals::PostgresRemoteGoalRepository;

use chrono::NaiveDateTime;
use k_ap::{FollowerStatus, RemoteActor};
use sqlx::{PgPool, Row};

pub(crate) fn datetime_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn status_to_str(status: &FollowerStatus) -> &'static str {
    match status {
        FollowerStatus::Pending => "pending",
        FollowerStatus::Accepted => "accepted",
        FollowerStatus::Rejected => "rejected",
    }
}

pub(crate) fn str_to_status(s: &str) -> FollowerStatus {
    match s {
        "accepted" => FollowerStatus::Accepted,
        "rejected" => FollowerStatus::Rejected,
        _ => FollowerStatus::Pending,
    }
}

pub(crate) fn pg_remote_actor(row: &sqlx::postgres::PgRow, url_col: &str) -> RemoteActor {
    RemoteActor {
        url: row.get(url_col),
        handle: row.try_get("handle").unwrap_or_default(),
        inbox_url: row.try_get("inbox_url").unwrap_or_default(),
        shared_inbox_url: row.try_get("shared_inbox_url").ok().flatten(),
        display_name: row.try_get("display_name").ok().flatten(),
        avatar_url: row.try_get("avatar_url").ok().flatten(),
        outbox_url: row.try_get("outbox_url").ok().flatten(),
        bio: row.try_get("bio").ok().flatten(),
        banner_url: row.try_get("banner_url").ok().flatten(),
        followers_url: row.try_get("followers_url").ok().flatten(),
        following_url: row.try_get("following_url").ok().flatten(),
        also_known_as: row
            .try_get::<Option<String>, _>("also_known_as")
            .ok()
            .flatten()
            .map(|s| {
                serde_json::from_str::<Vec<String>>(&s).unwrap_or_else(|e| {
                    tracing::warn!(raw = %s, error = %e, "failed to parse also_known_as JSON");
                    vec![s]
                })
            })
            .unwrap_or_default(),
        fetched_at: row.try_get("fetched_at").ok(),
    }
}

pub(crate) const PG_ACTOR_COLS: &str = "a.handle, a.inbox_url, a.shared_inbox_url, a.display_name, a.avatar_url, a.outbox_url, a.bio, a.banner_url, a.followers_url, a.following_url, a.also_known_as, a.fetched_at";

pub struct PostgresFederationRepository {
    pub(crate) pool: PgPool,
}

impl PostgresFederationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub fn wire(pool: PgPool) -> activitypub::FederationRepos {
    let fed = std::sync::Arc::new(PostgresFederationRepository::new(pool));
    (
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        std::sync::Arc::clone(&fed) as _,
        fed as _,
    )
}
