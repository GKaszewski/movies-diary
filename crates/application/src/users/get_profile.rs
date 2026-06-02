use crate::{
    context::AppContext,
    users::queries::{GetUserProfileQuery, ProfileView},
};
use chrono::Datelike;
use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, MonthActivity, SortDirection, UserStats, UserTrends,
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
    pub history: Option<Vec<MonthActivity>>,
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
            let history = group_by_month(all_entries);
            Ok(base(None, Some(history), None))
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
    if !is_own_profile {
        return (0, 0, vec![]);
    }
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

fn group_by_month(entries: Vec<DiaryEntry>) -> Vec<MonthActivity> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<(i32, u32), Vec<DiaryEntry>> = BTreeMap::new();
    for entry in entries {
        let watched_at = entry.review().watched_at();
        let year = watched_at.year();
        let month = watched_at.month();
        map.entry((year, month)).or_default().push(entry);
    }
    map.into_iter()
        .rev()
        .map(|((year, month), entries)| {
            let year_month = format!("{:04}-{:02}", year, month);
            MonthActivity {
                month_label: format_year_month_long(&year_month),
                count: entries.len() as i64,
                entries,
                year_month,
            }
        })
        .collect()
}

fn format_year_month_long(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 {
        return ym.to_string();
    }
    let month = match parts[1] {
        "01" => "January",
        "02" => "February",
        "03" => "March",
        "04" => "April",
        "05" => "May",
        "06" => "June",
        "07" => "July",
        "08" => "August",
        "09" => "September",
        "10" => "October",
        "11" => "November",
        "12" => "December",
        _ => parts[1],
    };
    format!("{} {}", month, parts[0])
}
