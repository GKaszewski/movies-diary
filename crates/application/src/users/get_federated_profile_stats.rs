//! The federated half of the former `get_profile` (ADR-0004's closed wart).
//! `build_federated_profile_response` (`http-axum/src/api/user_profile.rs`) calls this with a
//! synthetic `user_id` that has no row in the local `users` table — a federated
//! actor's identity is built entirely from the resolved `FederatedProfile`, so
//! this type carries no `identity` field at all: the old `get_profile` computed
//! one for this path anyway, filled with empty-string sentinels for `username`/
//! `handle`, and the handler never read it (see the closed wart paragraph in
//! `docs/adr/0004-instance-identity.md`). No type here can represent that
//! sentinel, because there is nothing to fill in.
//!
//! A federated actor also has no local pending-follow-request concept, so unlike
//! `get_local_profile` this never computes `pending_followers` — it would always
//! be empty here regardless of `is_own_profile`.

use crate::users::{
    deps::GetFederatedProfileStatsDeps,
    diary_filter::{feed_sort_to_direction, paged_user_filter},
    queries::{GetUserProfileQuery, ProfileView},
};
use domain::{
    errors::DomainError,
    models::{DiaryEntry, UserStats, UserTrends, collections::Paginated},
    value_objects::UserId,
};

pub struct FederatedProfileStats {
    pub stats: UserStats,
    pub entries: Option<Paginated<DiaryEntry>>,
    pub history: Option<Vec<DiaryEntry>>,
    pub trends: Option<UserTrends>,
    pub following_count: usize,
    pub followers_count: usize,
}

pub async fn execute(
    deps: &GetFederatedProfileStatsDeps,
    query: GetUserProfileQuery,
) -> Result<FederatedProfileStats, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let stats = deps.stats.get_user_stats(&user_id).await?;

    let following_count = deps
        .social_query
        .count_following(&user_id)
        .await
        .unwrap_or(0);
    let followers_count = deps
        .social_query
        .count_followers(&user_id)
        .await
        .unwrap_or(0);

    let base = |entries, history, trends| FederatedProfileStats {
        stats,
        entries,
        history,
        trends,
        following_count,
        followers_count,
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
