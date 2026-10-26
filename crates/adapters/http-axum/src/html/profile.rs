use crate::html::helpers::build_page_context;
use crate::html::templates::{
    EmbedProfileTemplate, MonthlyRatingRow, ProfileTemplate, RemoteActorData, bar_height_px,
    build_heatmap, build_page_items,
};
use crate::{
    extractors::OptionalCookieUser, html::csrf::CsrfToken, html::render::render_page,
    state::AppState,
};
use application::users::get_local_profile;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use domain::errors::DomainError;
use std::str::FromStr;
use uuid::Uuid;

const DEFAULT_PAGE_LIMIT: u32 = 5;

struct PaginationInfo {
    offset: u32,
    has_more: bool,
    limit: u32,
    page_items: Vec<crate::html::templates::PageItem>,
}

fn compute_pagination(
    entries: Option<&domain::models::collections::Paginated<domain::models::DiaryEntry>>,
) -> PaginationInfo {
    let (offset, has_more, limit) = entries
        .map(|e| {
            let has_more = (e.offset as u64).saturating_add(e.limit as u64) < e.total_count;
            (e.offset, has_more, e.limit)
        })
        .unwrap_or((0, false, DEFAULT_PAGE_LIMIT));
    let total = entries.map(|e| e.total_count as u32).unwrap_or(0);
    let total_pages = total
        .saturating_add(limit.saturating_sub(1))
        .checked_div(limit)
        .unwrap_or(1);
    let current_page = offset.checked_div(limit).unwrap_or(0);
    let page_items = build_page_items(total_pages, current_page);
    PaginationInfo {
        offset,
        has_more,
        limit,
        page_items,
    }
}

struct StatsDisplay {
    avg_rating: String,
    favorite_director: String,
    most_active_month: String,
}

fn build_stats_display(stats: &domain::models::UserStats) -> StatsDisplay {
    StatsDisplay {
        avg_rating: stats
            .avg_rating
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "\u{2014}".to_string()),
        favorite_director: stats
            .favorite_director
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_string()),
        most_active_month: stats
            .most_active_month
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_string()),
    }
}

fn build_monthly_rating_rows<'a>(
    trends: Option<&'a domain::models::UserTrends>,
) -> Vec<MonthlyRatingRow<'a>> {
    trends
        .map(|t| {
            t.monthly_ratings
                .iter()
                .map(|r| MonthlyRatingRow {
                    rating: r,
                    bar_height_px: bar_height_px(r.avg_rating),
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn fetch_profile_goals(
    state: &AppState,
    user_id: Uuid,
) -> Vec<crate::html::templates::GoalViewData> {
    let goals_list = application::goals::list::execute(
        &state.app_ctx.deps.goals.query,
        application::goals::queries::ListGoalsQuery { user_id },
    )
    .await
    .unwrap_or_default();
    goals_list
        .iter()
        .map(|g| crate::html::templates::GoalViewData {
            year: g.goal.year(),
            target_count: g.goal.target_count(),
            current_count: g.current_count,
            percentage: g.percentage().round(),
            is_complete: g.is_complete(),
        })
        .collect()
}

pub async fn get_user_profile_html(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<crate::html::forms::ProfileQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    // Content negotiation: AP clients request application/activity+json
    if let Some(response) =
        crate::federation::actor_document_response(&state, &headers, profile_user_uuid).await
    {
        return response;
    }

    let mut ctx = build_page_context(&state, user_id.clone(), csrf.0).await;
    let view_str = params.view.as_deref().unwrap_or("recent");
    let profile_view = match application::users::queries::ProfileView::from_str(view_str) {
        Ok(v) => v,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "invalid view parameter",
            )
                .into_response();
        }
    };

    let sort_by_str = match params.sort_by.as_str() {
        "date_asc" => "date_asc",
        "rating" => "rating",
        "rating_asc" => "rating_asc",
        _ => "date",
    };

    let is_own_profile = user_id
        .as_ref()
        .map(|u| u.value() == profile_user_uuid)
        .unwrap_or(false);

    let query = application::users::queries::GetUserProfileQuery {
        user_id: profile_user_uuid,
        view: profile_view,
        limit: params.limit,
        offset: params.offset,
        sort_by: sort_by_str.parse().unwrap_or_default(),
        search: if params.search.is_empty() {
            None
        } else {
            Some(params.search.clone())
        },
        is_own_profile,
        include_remote: false,
    };

    match get_local_profile::execute(&state.app_ctx.deps.users.get_local_profile, query).await {
        Ok(profile) => {
            ctx.page_title = format!("{}'s Diary — Movies Diary", profile.identity.username);
            ctx.canonical_url = format!(
                "{}/users/{}",
                state.app_ctx.config.base_url, profile_user_uuid
            );

            let pag = compute_pagination(profile.entries.as_ref());
            if !is_own_profile {
                ctx.page_rss_url = Some(format!("/users/{}/feed.rss", profile_user_uuid));
            }
            let display_name = profile
                .identity
                .email
                .split('@')
                .next()
                .unwrap_or("?")
                .to_string();
            let stats_disp = build_stats_display(&profile.stats);
            let history = profile.history.map(application::users::group_by_month);
            let heatmap = history.as_deref().map(build_heatmap).unwrap_or_default();
            let monthly_rating_rows = build_monthly_rating_rows(profile.trends.as_ref());
            let pending_followers: Vec<RemoteActorData> = profile
                .pending_followers
                .iter()
                .map(crate::html::mappers::users::pending_follower_data)
                .collect();
            if params.embed {
                let profile_url = format!(
                    "{}/users/{}",
                    state.app_ctx.config.base_url, profile_user_uuid
                );
                let response = render_page(EmbedProfileTemplate {
                    profile_display_name: display_name,
                    profile_user_id: profile_user_uuid,
                    profile_url,
                    stats: &profile.stats,
                    avg_rating_display: stats_disp.avg_rating,
                    favorite_director_display: stats_disp.favorite_director,
                    most_active_month_display: stats_disp.most_active_month,
                    view: profile_view.as_str(),
                    entries: profile.entries.as_ref(),
                    current_offset: pag.offset,
                    has_more: pag.has_more,
                    limit: pag.limit,
                    history: history.as_ref(),
                    trends: profile.trends.as_ref(),
                    monthly_rating_rows,
                    heatmap,
                    page_items: pag.page_items,
                    sort_by: sort_by_str.to_string(),
                });
                let mut resp = response.into_response();
                resp.headers_mut().remove("x-frame-options");
                resp
            } else {
                let goals = fetch_profile_goals(&state, profile_user_uuid).await;
                render_page(ProfileTemplate {
                    ctx: &ctx,
                    profile_display_name: display_name,
                    profile_user_id: profile_user_uuid,
                    stats: &profile.stats,
                    avg_rating_display: stats_disp.avg_rating,
                    favorite_director_display: stats_disp.favorite_director,
                    most_active_month_display: stats_disp.most_active_month,
                    view: profile_view.as_str(),
                    entries: profile.entries.as_ref(),
                    current_offset: pag.offset,
                    has_more: pag.has_more,
                    limit: pag.limit,
                    history: history.as_ref(),
                    trends: profile.trends.as_ref(),
                    monthly_rating_rows,
                    heatmap,
                    page_items: pag.page_items,
                    is_own_profile,
                    is_service_account: domain::value_objects::UserId::from_uuid(profile_user_uuid)
                        .is_service_actor(),
                    error: params.error,
                    following_count: profile.following_count,
                    followers_count: profile.followers_count,
                    pending_followers,
                    sort_by: sort_by_str.to_string(),
                    search: params.search.clone(),
                    goals,
                })
                .into_response()
            }
        }
        Err(DomainError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}
