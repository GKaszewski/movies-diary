use domain::value_objects::{FollowStatus, InstanceIdentity, SocialActor, SocialIdentity};
use sqlx::Row;

pub(super) fn follow_status_to_str(status: &FollowStatus) -> &'static str {
    match status {
        FollowStatus::Pending => "pending",
        FollowStatus::Accepted => "accepted",
        FollowStatus::Rejected => "rejected",
    }
}

pub(super) fn follow_status_from_str(status: &str) -> Option<FollowStatus> {
    match status {
        "pending" => Some(FollowStatus::Pending),
        "accepted" => Some(FollowStatus::Accepted),
        "rejected" => Some(FollowStatus::Rejected),
        _ => None,
    }
}

pub(super) fn social_actor_from_row(
    row: &sqlx::sqlite::SqliteRow,
    instance: &InstanceIdentity,
) -> SocialActor {
    let actor_url: String = row.get("remote_actor_url");
    let identity = instance.identify(&actor_url);

    let (handle, display_name, avatar_url) = match &identity {
        SocialIdentity::Local(_) => {
            let username: Option<String> = row.try_get("local_username").ok().flatten();
            let display: Option<String> = row.try_get("local_display").ok().flatten();
            let avatar: Option<String> = row
                .try_get::<Option<String>, _>("local_avatar_path")
                .ok()
                .flatten()
                .map(|p| instance.image_url_for(&p));
            let handle = username
                .as_deref()
                .map(|u| instance.handle_for(u))
                .unwrap_or_else(|| actor_url.clone());
            (handle, display, avatar)
        }
        SocialIdentity::Remote { .. } => {
            let handle: String = row
                .try_get::<Option<String>, _>("remote_handle")
                .ok()
                .flatten()
                .unwrap_or_else(|| actor_url.clone());
            let display: Option<String> = row.try_get("remote_display").ok().flatten();
            let avatar: Option<String> = row.try_get("remote_avatar").ok().flatten();
            (handle, display, avatar)
        }
    };

    SocialActor {
        identity,
        handle,
        display_name,
        avatar_url,
    }
}
