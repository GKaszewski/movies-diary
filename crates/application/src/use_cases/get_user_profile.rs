use domain::{
    errors::DomainError,
    models::{
        DiaryEntry, DiaryFilter, MonthActivity, SortDirection, UserStats, UserTrends,
        collections::{PageParams, Paginated},
    },
    value_objects::UserId,
};
use crate::{context::AppContext, queries::GetUserProfileQuery};

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
    let stats = ctx.repository.get_user_stats(&user_id).await?;

    match query.view.as_str() {
        "history" => {
            // V1: loads all entries into memory. Personal diaries are bounded in size;
            // spec calls for showing every movie grouped by month, so full load is intentional.
            let all_entries = ctx.repository.get_user_history(&user_id).await?;
            let history = group_by_month(all_entries);
            Ok(UserProfileData { stats, entries: None, history: Some(history), trends: None })
        }
        "trends" => {
            let trends = ctx.repository.get_user_trends(&user_id).await?;
            Ok(UserProfileData { stats, entries: None, history: None, trends: Some(trends) })
        }
        "ratings" => {
            let page = PageParams::new(query.limit, query.offset)?;
            let filter = DiaryFilter {
                sort_by: SortDirection::ByRatingDesc,
                page,
                movie_id: None,
                user_id: Some(user_id),
            };
            let entries = ctx.repository.query_diary(&filter).await?;
            Ok(UserProfileData { stats, entries: Some(entries), history: None, trends: None })
        }
        "recent" => {
            let page = PageParams::new(query.limit, query.offset)?;
            let filter = DiaryFilter {
                sort_by: SortDirection::Descending,
                page,
                movie_id: None,
                user_id: Some(user_id),
            };
            let entries = ctx.repository.query_diary(&filter).await?;
            Ok(UserProfileData { stats, entries: Some(entries), history: None, trends: None })
        }
        other => Err(DomainError::ValidationError(format!("unknown view: {}", other))),
    }
}

fn group_by_month(entries: Vec<DiaryEntry>) -> Vec<MonthActivity> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<DiaryEntry>> = BTreeMap::new();
    for entry in entries {
        let ym = entry.review().watched_at().format("%Y-%m").to_string();
        map.entry(ym).or_default().push(entry);
    }
    let mut result: Vec<MonthActivity> = map
        .into_iter()
        .map(|(ym, entries)| MonthActivity {
            month_label: format_year_month_long(&ym),
            count: entries.len() as i64,
            entries,
            year_month: ym,
        })
        .collect();
    result.reverse();
    result
}

fn format_year_month_long(ym: &str) -> String {
    let parts: Vec<&str> = ym.splitn(2, '-').collect();
    if parts.len() != 2 { return ym.to_string(); }
    let month = match parts[1] {
        "01" => "January", "02" => "February", "03" => "March", "04" => "April",
        "05" => "May", "06" => "June", "07" => "July", "08" => "August",
        "09" => "September", "10" => "October", "11" => "November", "12" => "December",
        _ => parts[1],
    };
    format!("{} {}", month, parts[0])
}
