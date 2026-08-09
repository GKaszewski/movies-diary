//! The local half of the former `get_profile` (ADR-0004's closed wart). Unlike
//! the old unified function, a user id with no local row is `Err(NotFound)` here
//! — there is no federated fallback inside this function, and no empty-string
//! sentinel for `identity`, because `LocalProfileData::identity` is always fully
//! populated by construction.

use crate::users::{
    deps::GetLocalProfileDeps,
    diary_filter::{feed_sort_to_direction, paged_user_filter},
    queries::{GetUserProfileQuery, ProfileView},
};
use domain::{
    errors::DomainError,
    models::{DiaryEntry, UserStats, UserTrends, collections::Paginated},
    value_objects::{InstanceIdentity, UserId},
};

pub struct PendingFollowerView {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct ProfileIdentity {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub handle: String,
    pub actor_url: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    /// Only the HTML profile handler reads this (it derives its "display name"
    /// from the email's local part, not from `username` — see
    /// `handlers/users.rs::get_user_profile_html`). Sourced from the same local
    /// `User` row as every other identity field.
    pub email: String,
}

pub struct LocalProfileData {
    pub stats: UserStats,
    pub entries: Option<Paginated<DiaryEntry>>,
    pub history: Option<Vec<DiaryEntry>>,
    pub trends: Option<UserTrends>,
    pub following_count: usize,
    pub followers_count: usize,
    pub pending_followers: Vec<PendingFollowerView>,
    pub identity: ProfileIdentity,
}

pub async fn execute(
    deps: &GetLocalProfileDeps,
    query: GetUserProfileQuery,
) -> Result<LocalProfileData, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let user = deps
        .user
        .find_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound(format!("user {}", query.user_id)))?;

    let stats = deps.stats.get_user_stats(&user_id).await?;

    let (following_count, followers_count, pending_followers) =
        load_social_counts(deps, &user_id, query.is_own_profile, &deps.instance).await;

    let identity = ProfileIdentity {
        username: user.username().value().to_string(),
        display_name: user.display_name().map(str::to_string),
        bio: user.bio().map(str::to_string),
        handle: deps.instance.handle_for(user.username().value()),
        actor_url: deps.instance.actor_url_for(&user_id),
        avatar_url: user.avatar_path().map(|p| deps.instance.image_url_for(p)),
        banner_url: user.banner_path().map(|p| deps.instance.image_url_for(p)),
        email: user.email().value().to_string(),
    };

    let base = |entries, history, trends| LocalProfileData {
        stats,
        entries,
        history,
        trends,
        following_count,
        followers_count,
        pending_followers,
        identity,
    };

    match query.view {
        ProfileView::History => {
            let all_entries = deps.diary.get_user_history(&user_id).await?;
            Ok(base(None, Some(all_entries), None))
        }
        ProfileView::Trends => {
            let trends = deps.stats.get_user_trends(&user_id).await?;
            Ok(base(None, None, Some(trends)))
        }
        ProfileView::Ratings | ProfileView::Recent => {
            let sort_direction = feed_sort_to_direction(query.sort_by);
            let filter = paged_user_filter(
                user_id,
                sort_direction,
                query.limit,
                query.offset,
                query.search.clone(),
                query.include_remote,
            )?;
            let entries = deps.diary.query_diary(&filter).await?;
            Ok(base(Some(entries), None, None))
        }
    }
}

async fn load_social_counts(
    deps: &GetLocalProfileDeps,
    user_id: &UserId,
    is_own_profile: bool,
    instance: &InstanceIdentity,
) -> (usize, usize, Vec<PendingFollowerView>) {
    let following = deps
        .social_query
        .count_following(user_id)
        .await
        .unwrap_or(0);
    let followers = deps
        .social_query
        .count_followers(user_id)
        .await
        .unwrap_or(0);
    if !is_own_profile {
        return (following, followers, vec![]);
    }
    let pending = deps
        .social_query
        .get_pending_followers(user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let url = instance.actor_url_of(&p.identity);
            PendingFollowerView {
                url,
                handle: p.handle,
                display_name: p.display_name,
                avatar_url: p.avatar_url,
            }
        })
        .collect();
    (following, followers, pending)
}

#[cfg(test)]
#[path = "tests/get_local_profile.rs"]
mod tests;
