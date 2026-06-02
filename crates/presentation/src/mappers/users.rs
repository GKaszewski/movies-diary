use application::users::get_profile::PendingFollowerView;
use domain::models::UserSummary;
use domain::ports::RemoteActorInfo;
use template_askama::{RemoteActorData, RemoteActorDisplay, UserSummaryView};

pub fn user_summary_view(u: &UserSummary) -> UserSummaryView {
    let name = u.email().split('@').next().unwrap_or("?").to_string();
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
