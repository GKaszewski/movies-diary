use crate::{
    context::AppContext,
    queries::{GetUserProfileQuery, ProfileView},
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

pub struct UserProfileData {
    pub stats: UserStats,
    pub entries: Option<Paginated<DiaryEntry>>,
    pub history: Option<Vec<MonthActivity>>,
    pub trends: Option<UserTrends>,
}

pub async fn execute(
    ctx: &AppContext,
    query: GetUserProfileQuery,
) -> Result<UserProfileData, DomainError> {
    let user_id = UserId::from_uuid(query.user_id);
    let stats = ctx.stats_repository.get_user_stats(&user_id).await?;

    match query.view {
        ProfileView::History => {
            let all_entries = ctx.diary_repository.get_user_history(&user_id).await?;
            let history = group_by_month(all_entries);
            Ok(UserProfileData {
                stats,
                entries: None,
                history: Some(history),
                trends: None,
            })
        }
        ProfileView::Trends => {
            let trends = ctx.stats_repository.get_user_trends(&user_id).await?;
            Ok(UserProfileData {
                stats,
                entries: None,
                history: None,
                trends: Some(trends),
            })
        }
        ProfileView::Ratings => {
            let sort_direction = feed_sort_to_direction(query.sort_by);
            let filter = paged_user_filter(
                user_id,
                sort_direction,
                query.limit,
                query.offset,
                query.search.clone(),
            )?;
            let entries = ctx.diary_repository.query_diary(&filter).await?;
            Ok(UserProfileData {
                stats,
                entries: Some(entries),
                history: None,
                trends: None,
            })
        }
        ProfileView::Recent => {
            let sort_direction = feed_sort_to_direction(query.sort_by);
            let filter = paged_user_filter(
                user_id,
                sort_direction,
                query.limit,
                query.offset,
                query.search.clone(),
            )?;
            let entries = ctx.diary_repository.query_diary(&filter).await?;
            Ok(UserProfileData {
                stats,
                entries: Some(entries),
                history: None,
                trends: None,
            })
        }
    }
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
