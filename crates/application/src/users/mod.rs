pub mod commands;
pub mod delete_account;
pub mod deps;
pub mod get_current_profile;
pub mod get_profile;
pub mod get_settings;
pub mod get_users;
pub mod queries;
pub mod update_profile;
pub mod update_profile_fields;
pub mod update_settings;

use chrono::Datelike;
use domain::models::{DiaryEntry, MonthActivity};

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
