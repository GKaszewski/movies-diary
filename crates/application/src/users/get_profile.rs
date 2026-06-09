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

#[cfg(test)]
#[path = "tests/get_profile.rs"]
mod tests;

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn format_year_month_long_all_months() {
        assert_eq!(format_year_month_long("2024-01"), "January 2024");
        assert_eq!(format_year_month_long("2024-02"), "February 2024");
        assert_eq!(format_year_month_long("2024-03"), "March 2024");
        assert_eq!(format_year_month_long("2024-04"), "April 2024");
        assert_eq!(format_year_month_long("2024-05"), "May 2024");
        assert_eq!(format_year_month_long("2024-06"), "June 2024");
        assert_eq!(format_year_month_long("2024-07"), "July 2024");
        assert_eq!(format_year_month_long("2024-08"), "August 2024");
        assert_eq!(format_year_month_long("2024-09"), "September 2024");
        assert_eq!(format_year_month_long("2024-10"), "October 2024");
        assert_eq!(format_year_month_long("2024-11"), "November 2024");
        assert_eq!(format_year_month_long("2024-12"), "December 2024");
    }

    #[test]
    fn format_year_month_long_invalid() {
        assert_eq!(format_year_month_long("invalid"), "invalid");
        assert_eq!(format_year_month_long("2024-99"), "99 2024");
    }

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
    fn group_by_month_empty() {
        assert!(group_by_month(vec![]).is_empty());
    }

    #[test]
    fn group_by_month_groups_entries() {
        use chrono::NaiveDateTime;
        use domain::models::{Movie, Review};
        use domain::value_objects::{MovieId, MovieTitle, Rating, ReleaseYear, UserId};

        let movie = Movie::from_persistence(
            MovieId::generate(),
            None,
            MovieTitle::new("Test".into()).unwrap(),
            ReleaseYear::new(2024).unwrap(),
            None,
            None,
        );
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());

        let jan =
            NaiveDateTime::parse_from_str("2024-01-15 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let jan2 =
            NaiveDateTime::parse_from_str("2024-01-20 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let mar =
            NaiveDateTime::parse_from_str("2024-03-05 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let r1 = Review::new(
            movie.id().clone(),
            uid.clone(),
            Rating::new(4).unwrap(),
            None,
            jan,
        )
        .unwrap();
        let r2 = Review::new(
            movie.id().clone(),
            uid.clone(),
            Rating::new(3).unwrap(),
            None,
            jan2,
        )
        .unwrap();
        let r3 = Review::new(
            movie.id().clone(),
            uid.clone(),
            Rating::new(5).unwrap(),
            None,
            mar,
        )
        .unwrap();

        let entries = vec![
            DiaryEntry::new(movie.clone(), r1),
            DiaryEntry::new(movie.clone(), r2),
            DiaryEntry::new(movie.clone(), r3),
        ];

        let result = group_by_month(entries);
        // Reversed: March first, then January
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].month_label, "March 2024");
        assert_eq!(result[0].count, 1);
        assert_eq!(result[1].month_label, "January 2024");
        assert_eq!(result[1].count, 2);
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
