mod activity;
mod actor;
pub mod ap_content;
mod blocklist;
mod follow;
pub mod remote_goals;
mod review;
mod social;
mod watchlist;

pub use ap_content::SqliteApContentQuery;
pub use remote_goals::SqliteRemoteGoalRepository;

use chrono::NaiveDateTime;
use k_ap::{FollowerStatus, RemoteActor};
use sqlx::SqlitePool;

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

pub(crate) fn remote_actor_from_row(row: &sqlx::sqlite::SqliteRow, url_col: &str) -> RemoteActor {
    use sqlx::Row;
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
        fetched_at: row
            .try_get::<Option<String>, _>("fetched_at")
            .ok()
            .flatten()
            .and_then(|s| {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| ndt.and_utc())
                    .or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    })
                    .ok()
            }),
    }
}

pub struct SqliteFederationRepository {
    pub(crate) pool: SqlitePool,
}

impl SqliteFederationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn wire(pool: SqlitePool) -> activitypub::FederationRepos {
    let fed = std::sync::Arc::new(SqliteFederationRepository::new(pool));
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

#[cfg(test)]
#[path = "tests/outbox_url.rs"]
mod outbox_url_tests;

#[cfg(test)]
#[path = "tests/actor_block_tests.rs"]
mod actor_block_tests;

#[cfg(test)]
#[path = "tests/domain_block_tests.rs"]
mod domain_block_tests;

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
