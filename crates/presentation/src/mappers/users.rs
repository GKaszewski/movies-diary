use application::users::get_profile::PendingFollowerView;
use chrono::Datelike;
use domain::models::{DiaryEntry, MonthActivity, UserSummary};
use domain::ports::RemoteActorInfo;
use template_askama::{RemoteActorData, RemoteActorDisplay, UserSummaryView};

pub fn user_summary_view(u: &UserSummary) -> UserSummaryView {
    let name = u
        .display_name()
        .map(String::from)
        .unwrap_or_else(|| u.username().to_string());
    let initial = name.chars().next().unwrap_or('?').to_ascii_uppercase();
    let avg_display = u
        .avg_rating
        .map(|r| format!("{:.1}", r))
        .unwrap_or_else(|| "\u{2014}".to_string());
    let avatar_url = u.avatar_path.as_ref().map(|p| format!("/images/{}", p));
    UserSummaryView {
        user_id: u.user_id.value(),
        display_name: name,
        initial,
        avg_rating_display: avg_display,
        total_movies: u.total_movies,
        avatar_url,
    }
}

pub fn remote_actor_display(a: &RemoteActorInfo) -> RemoteActorDisplay {
    let display = a.display_name.clone().unwrap_or_else(|| a.handle.clone());
    let initial = display.chars().next().unwrap_or('?').to_ascii_uppercase();
    RemoteActorDisplay {
        handle: a.handle.clone(),
        display_name: display,
        initial,
        url: a.url.clone(),
    }
}

pub fn pending_follower_data(p: &PendingFollowerView) -> RemoteActorData {
    RemoteActorData {
        handle: p.handle.clone(),
        url: p.url.clone(),
        display_name: p.display_name.clone(),
        avatar_url: p.avatar_url.clone(),
    }
}

pub fn group_by_month(entries: Vec<DiaryEntry>) -> Vec<MonthActivity> {
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
