use crate::{
    context::AppContext,
    users::queries::{GetUserProfileQuery, ProfileView},
};
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, SortDirection, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    ports::FeedSortBy,
    value_objects::UserId,
};

pub struct PendingFollowerView {
    pub url: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct UserProfileData {
    pub stats: UserStats,
    pub entries: Option<Paginated<DiaryEntry>>,
    pub history: Option<Vec<DiaryEntry>>,
    pub trends: Option<UserTrends>,
    pub following_count: usize,
    pub followers_count: usize,
    pub pending_followers: Vec<PendingFollowerView>,
}

pub async fn execute(
    ctx: &AppContext,
    query: GetUserProfileQuery,
) -> Result<UserProfileData, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let stats = ctx.repos.stats.get_user_stats(&user_id).await?;

    let (following_count, followers_count, pending_followers) =
        load_social_counts(ctx, query.user_id, query.is_own_profile).await;

    let base = |entries, history, trends| UserProfileData {
        stats,
        entries,
        history,
        trends,
        following_count,
        followers_count,
        pending_followers,
    };

    match query.view {
        ProfileView::History => {
            let all_entries = ctx.repos.diary.get_user_history(&user_id).await?;
            Ok(base(None, Some(all_entries), None))
        }
        ProfileView::Trends => {
            let trends = ctx.repos.stats.get_user_trends(&user_id).await?;
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
            )?;
            let entries = ctx.repos.diary.query_diary(&filter).await?;
            Ok(base(Some(entries), None, None))
        }
    }
}

async fn load_social_counts(
    ctx: &AppContext,
    user_id: uuid::Uuid,
    is_own_profile: bool,
) -> (usize, usize, Vec<PendingFollowerView>) {
    let following = ctx
        .repos
        .social_query
        .count_following(user_id)
        .await
        .unwrap_or(0);
    let followers = ctx
        .repos
        .social_query
        .count_accepted_followers(user_id)
        .await
        .unwrap_or(0);
    if !is_own_profile {
        return (following, followers, vec![]);
    }
    let pending = ctx
        .repos
        .social_query
        .get_pending_followers(user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| PendingFollowerView {
            url: p.url,
            handle: p.handle,
            display_name: p.display_name,
            avatar_url: p.avatar_url,
        })
        .collect();
    (following, followers, pending)
}

fn feed_sort_to_direction(sort_by: FeedSortBy) -> SortDirection {
    match sort_by {
        FeedSortBy::Date => SortDirection::Descending,
        FeedSortBy::DateAsc => SortDirection::Ascending,
        FeedSortBy::Rating => SortDirection::ByRatingDesc,
        FeedSortBy::RatingAsc => SortDirection::ByRatingAsc,
    }
}

fn paged_user_filter(
    user_id: UserId,
    sort_by: SortDirection,
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
) -> Result<DiaryFilter, DomainError> {
    let page = PageParams::new(limit, offset)?;
    Ok(DiaryFilter {
        sort_by,
        page,
        movie_id: None,
        user_id: Some(user_id),
        search,
    })
}

#[cfg(test)]
#[path = "tests/get_profile.rs"]
mod tests;

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn feed_sort_to_direction_all_variants() {
        use domain::ports::FeedSortBy;
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::Date),
            SortDirection::Descending
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::DateAsc),
            SortDirection::Ascending
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::Rating),
            SortDirection::ByRatingDesc
        ));
        assert!(matches!(
            feed_sort_to_direction(FeedSortBy::RatingAsc),
            SortDirection::ByRatingAsc
        ));
    }

    #[test]
    fn paged_user_filter_builds_correctly() {
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());
        let filter = paged_user_filter(
            uid.clone(),
            SortDirection::Descending,
            Some(20),
            Some(5),
            Some("blade".into()),
        )
        .unwrap();

        assert_eq!(filter.user_id.unwrap().value(), uid.value());
        assert_eq!(filter.search.as_deref(), Some("blade"));
    }
}
