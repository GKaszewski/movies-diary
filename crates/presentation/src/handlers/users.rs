use std::str::FromStr;

use axum::{
    Json,
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use application::users::{
    get_federated_profile, get_federated_profile_stats, get_local_profile,
    get_profile_settings as get_profile_settings_uc, get_users,
    queries::{GetUserProfileQuery, GetUsersQuery},
    resolve_username_to_id, update_profile, update_profile_fields,
};
use domain::{errors::DomainError, value_objects::Username};

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, OptionalCookieUser, RequiredCookieUser},
    render::render_page,
    state::AppState,
};
use api_types::{
    ActorStatDto, DiaryResponse, DirectorStatDto, GenreStatDto, MonthActivityDto, MonthlyRatingDto,
    ProfileResponse, UserProfileQueryParams, UserProfileResponse, UserStatsDto, UserSummaryDto,
    UserTrendsDto, UsersResponse, WatchMediumStatDto,
};
use template_askama::{
    EmbedProfileTemplate, MonthlyRatingRow, ProfileSettingsTemplate, ProfileTemplate,
    RemoteActorData, RemoteActorDisplay, UserSummaryView, UsersTemplate, bar_height_px,
    build_heatmap, build_page_items,
};

use super::goals::goal_with_progress_to_dto;
use super::helpers::build_page_context;

// ── API ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/profile",
    responses(
        (status = 200, body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> Result<Json<ProfileResponse>, ApiError> {
    let profile = application::users::get_current_profile::execute(
        &state.app_ctx.deps.users.get_current_profile,
        application::users::queries::GetCurrentProfileQuery {
            user_id: user_id.value(),
        },
    )
    .await?;
    let instance = &state.app_ctx.instance;
    Ok(Json(ProfileResponse {
        profile: api_types::UserProfileBase {
            username: profile.username,
            display_name: profile.display_name,
            bio: profile.bio,
            avatar_url: profile.avatar_path.map(|p| instance.image_url_for(&p)),
            banner_url: profile.banner_path.map(|p| instance.image_url_for(&p)),
        },
        also_known_as: profile.also_known_as,
        fields: profile
            .fields
            .into_iter()
            .map(|f| api_types::ProfileFieldDto {
                name: f.name,
                value: f.value,
            })
            .collect(),
        role: profile.role,
    }))
}

#[utoipa::path(
    put, path = "/api/v1/profile",
    responses(
        (status = 204, description = "Profile updated"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: Multipart,
) -> impl IntoResponse {
    let data = super::helpers::parse_profile_multipart(multipart).await;

    let cmd = application::users::commands::UpdateProfileCommand {
        user_id: user_id.value(),
        display_name: data.display_name,
        bio: data.bio,
        avatar_bytes: data.avatar_bytes,
        avatar_content_type: data.avatar_content_type,
        banner_bytes: data.banner_bytes,
        banner_content_type: data.banner_content_type,
        also_known_as: data.also_known_as,
    };

    match update_profile::execute(&state.app_ctx.deps.users.update_profile, cmd).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}

#[utoipa::path(
    put, path = "/api/v1/profile/fields",
    request_body = api_types::UpdateProfileFieldsRequest,
    responses(
        (status = 204, description = "Profile fields updated"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_fields_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let raw_fields = match body.get("fields").and_then(|f| f.as_array()) {
        Some(arr) => arr.clone(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let fields: Vec<domain::models::ProfileField> = raw_fields
        .iter()
        .filter_map(|f| {
            let name = f.get("name").and_then(|n| n.as_str())?.to_string();
            let value = f.get("value").and_then(|v| v.as_str())?.to_string();
            Some(domain::models::ProfileField { name, value })
        })
        .collect();

    let cmd = application::users::commands::UpdateProfileFieldsCommand {
        user_id: user_id.value(),
        fields,
    };

    match update_profile_fields::execute(&state.app_ctx.deps.users.update_profile_fields, cmd).await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => crate::errors::domain_error_response(e),
    }
}

#[utoipa::path(
    get, path = "/api/v1/users",
    responses((status = 200, body = UsersResponse)),
)]
pub async fn list_users(State(state): State<AppState>) -> Result<Json<UsersResponse>, ApiError> {
    let result =
        get_users::execute(&state.app_ctx.deps.users.get_users_list, GetUsersQuery).await?;
    Ok(Json(UsersResponse {
        users: result
            .users
            .iter()
            .map(|u| UserSummaryDto {
                id: u.user_id.value(),
                email: u.email().to_string(),
                username: u.username().to_string(),
                display_name: u.display_name().map(String::from),
                total_movies: u.total_movies,
                avg_rating: u.avg_rating,
            })
            .collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID"),
        UserProfileQueryParams,
    ),
    responses(
        (status = 200, body = UserProfileResponse),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    AuthenticatedUser(viewer_id): AuthenticatedUser,
    Path(user_id): Path<Uuid>,
    Query(params): Query<UserProfileQueryParams>,
) -> impl IntoResponse {
    let view_str = params.view.as_deref().unwrap_or("recent");
    let profile_view = match application::users::queries::ProfileView::from_str(view_str) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let profile = match get_local_profile::execute(
        &state.app_ctx.deps.users.get_local_profile,
        GetUserProfileQuery {
            user_id,
            view: profile_view,
            limit: params.limit,
            offset: params.offset,
            sort_by: domain::models::FeedSortBy::Date,
            search: params.search.clone(),
            is_own_profile: viewer_id.value() == user_id,
            include_remote: false,
        },
    )
    .await
    {
        Ok(p) => p,
        Err(DomainError::NotFound(_)) => {
            if let Ok(Some(fed)) = get_federated_profile::execute(
                &state.app_ctx.deps.users.get_federated_profile,
                user_id,
            )
            .await
            {
                return build_federated_profile_response(
                    &state,
                    user_id,
                    fed,
                    profile_view,
                    &params,
                )
                .await;
            }
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => return crate::errors::domain_error_response(e),
    };

    let view_data = if let Some(p) = profile.entries {
        Some(api_types::ProfileViewData::Entries {
            entries: DiaryResponse {
                items: p
                    .items
                    .iter()
                    .map(crate::mappers::movies::entry_to_dto)
                    .collect(),
                total_count: p.total_count,
                limit: p.limit,
                offset: p.offset,
            },
        })
    } else if let Some(h) = profile.history {
        Some(api_types::ProfileViewData::History {
            history: application::users::group_by_month(h)
                .into_iter()
                .map(|m| MonthActivityDto {
                    year_month: m.year_month,
                    month_label: m.month_label,
                    count: m.count,
                    entries: m
                        .entries
                        .iter()
                        .map(crate::mappers::movies::entry_to_dto)
                        .collect(),
                })
                .collect(),
        })
    } else {
        profile.trends.map(|t| api_types::ProfileViewData::Trends {
            trends: trends_to_dto(t),
        })
    };

    Json(UserProfileResponse {
        user_id,
        profile: api_types::UserProfileBase {
            username: profile.identity.username.clone(),
            avatar_url: profile.identity.avatar_url.clone(),
            banner_url: profile.identity.banner_url.clone(),
            display_name: profile.identity.display_name.clone(),
            bio: profile.identity.bio.clone(),
        },
        stats: UserStatsDto {
            total_movies: profile.stats.total_movies,
            avg_rating: profile.stats.avg_rating,
            favorite_director: profile.stats.favorite_director,
            most_active_month: profile.stats.most_active_month,
        },
        following_count: profile.following_count,
        followers_count: profile.followers_count,
        view_data,
        goals: {
            let goals_list = application::goals::list::execute(
                &state.app_ctx.deps.goals.query,
                application::goals::queries::ListGoalsQuery { user_id },
            )
            .await
            .unwrap_or_default();
            if goals_list.is_empty() {
                None
            } else {
                Some(goals_list.iter().map(goal_with_progress_to_dto).collect())
            }
        },
        is_federated: false,
        handle: Some(profile.identity.handle.clone()),
        actor_url: Some(profile.identity.actor_url.clone()),
    })
    .into_response()
}

async fn build_federated_profile_response(
    state: &AppState,
    user_id: Uuid,
    fed: domain::models::FederatedProfile,
    profile_view: application::users::queries::ProfileView,
    params: &UserProfileQueryParams,
) -> axum::response::Response {
    let profile = match get_federated_profile_stats::execute(
        &state.app_ctx.deps.users.get_federated_profile_stats,
        GetUserProfileQuery {
            user_id,
            view: profile_view,
            limit: params.limit,
            offset: params.offset,
            sort_by: domain::models::FeedSortBy::Date,
            search: params.search.clone(),
            is_own_profile: false,
            include_remote: true,
        },
    )
    .await
    {
        Ok(p) => p,
        Err(e) => return crate::errors::domain_error_response(e),
    };

    let view_data = if let Some(p) = profile.entries {
        Some(api_types::ProfileViewData::Entries {
            entries: DiaryResponse {
                items: p
                    .items
                    .iter()
                    .map(crate::mappers::movies::entry_to_dto)
                    .collect(),
                total_count: p.total_count,
                limit: p.limit,
                offset: p.offset,
            },
        })
    } else {
        profile.trends.map(|t| api_types::ProfileViewData::Trends {
            trends: trends_to_dto(t),
        })
    };

    let username = fed
        .display_name
        .clone()
        .unwrap_or_else(|| fed.handle.clone());

    Json(UserProfileResponse {
        user_id,
        profile: api_types::UserProfileBase {
            username,
            avatar_url: fed.avatar_url,
            banner_url: fed.banner_url,
            display_name: fed.display_name,
            bio: fed.bio,
        },
        stats: UserStatsDto {
            total_movies: profile.stats.total_movies,
            avg_rating: profile.stats.avg_rating,
            favorite_director: profile.stats.favorite_director,
            most_active_month: profile.stats.most_active_month,
        },
        following_count: 0,
        followers_count: 0,
        view_data,
        goals: None,
        is_federated: true,
        handle: Some(fed.handle),
        actor_url: Some(fed.actor_url),
    })
    .into_response()
}

fn trends_to_dto(t: domain::models::UserTrends) -> UserTrendsDto {
    UserTrendsDto {
        monthly_ratings: t
            .monthly_ratings
            .into_iter()
            .map(|r| MonthlyRatingDto {
                year_month: r.year_month,
                month_label: r.month_label,
                avg_rating: r.avg_rating,
                count: r.count,
            })
            .collect(),
        top_directors: t
            .top_directors
            .into_iter()
            .map(|d| DirectorStatDto {
                director: d.director,
                count: d.count,
            })
            .collect(),
        max_director_count: t.max_director_count,
        top_genres: t
            .top_genres
            .into_iter()
            .map(|g| GenreStatDto {
                genre: g.genre,
                count: g.count,
            })
            .collect(),
        rating_distribution: t.rating_distribution,
        watch_medium_distribution: t
            .watch_medium_distribution
            .into_iter()
            .map(|m| WatchMediumStatDto {
                medium: m.medium,
                count: m.count,
            })
            .collect(),
        top_actors: t
            .top_actors
            .into_iter()
            .map(|a| ActorStatDto {
                name: a.name,
                profile_path: a.profile_path,
                movie_count: a.movie_count,
                score: a.score,
            })
            .collect(),
    }
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_users_list(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, user_id, csrf.0).await;
    ctx.page_title = "Members — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/users", state.app_ctx.config.base_url);

    match application::users::get_users::execute(
        &state.app_ctx.deps.users.get_users_list,
        application::users::queries::GetUsersQuery,
    )
    .await
    {
        Ok(result) => {
            let users: Vec<UserSummaryView> = result
                .users
                .iter()
                .map(crate::mappers::users::user_summary_view)
                .collect();
            let remote_actors: Vec<RemoteActorDisplay> = result
                .remote_actors
                .iter()
                .map(crate::mappers::users::remote_actor_display)
                .collect();
            render_page(UsersTemplate {
                users,
                ctx: &ctx,
                remote_actors,
            })
            .into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn get_user_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    let uname = match Username::new(username) {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    match resolve_username_to_id::execute(&state.app_ctx.deps.users.resolve_username, &uname).await
    {
        Ok(Some(uid)) => {
            axum::response::Redirect::permanent(&format!("/users/{}", uid.value())).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── Profile helpers (private) ───────────────────────────────────────────────

struct PaginationInfo {
    offset: u32,
    has_more: bool,
    limit: u32,
    page_items: Vec<template_askama::PageItem>,
}

fn compute_pagination(
    entries: Option<&domain::models::collections::Paginated<domain::models::DiaryEntry>>,
) -> PaginationInfo {
    let (offset, has_more, limit) = entries
        .map(|e| {
            let has_more = (e.offset as u64).saturating_add(e.limit as u64) < e.total_count;
            (e.offset, has_more, e.limit)
        })
        .unwrap_or((0, false, super::DEFAULT_PAGE_LIMIT));
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
) -> Vec<template_askama::GoalViewData> {
    let goals_list = application::goals::list::execute(
        &state.app_ctx.deps.goals.query,
        application::goals::queries::ListGoalsQuery { user_id },
    )
    .await
    .unwrap_or_default();
    goals_list
        .iter()
        .map(|g| template_askama::GoalViewData {
            year: g.goal.year(),
            target_count: g.goal.target_count(),
            current_count: g.current_count,
            percentage: g.percentage().round(),
            is_complete: g.is_complete(),
        })
        .collect()
}

// ── Handler ─────────────────────────────────────────────────────────────────

pub async fn get_user_profile_html(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<crate::forms::ProfileQueryParams>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    // Content negotiation: AP clients request application/activity+json
    #[cfg(feature = "federation")]
    {
        let accept = headers
            .get(axum::http::header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
            return match state
                .app_ctx
                .ap_document
                .actor_json(&profile_user_uuid.to_string())
                .await
            {
                Ok(json) => (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "application/activity+json",
                    )],
                    json,
                )
                    .into_response(),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            };
        }
    }

    #[cfg(not(feature = "federation"))]
    let _ = &headers;

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
                .map(crate::mappers::users::pending_follower_data)
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

#[derive(serde::Deserialize, Default)]
pub struct SavedQuery {
    pub saved: Option<String>,
}

pub async fn get_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<SavedQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Profile Settings — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/settings/profile", state.app_ctx.config.base_url);

    let settings = match get_profile_settings_uc::execute(
        &state.app_ctx.deps.users.get_profile_settings,
        user_id.value(),
    )
    .await
    {
        Ok(s) => s,
        Err(DomainError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return crate::errors::domain_error_response(e),
    };

    let saved = params.saved.as_deref() == Some("1");

    render_page(ProfileSettingsTemplate {
        ctx: &ctx,
        bio: settings.bio.as_deref(),
        avatar_url: settings.avatar_url.as_deref(),
        banner_url: settings.banner_url.as_deref(),
        also_known_as: settings.also_known_as.as_deref(),
        profile_fields: &settings.fields,
        saved,
        embed_url: format!(
            "{}/users/{}?embed=true",
            state.app_ctx.config.base_url,
            user_id.value()
        ),
    })
    .into_response()
}

pub async fn post_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let data = super::helpers::parse_profile_multipart(multipart).await;

    let cmd = application::users::commands::UpdateProfileCommand {
        user_id: user_id.value(),
        display_name: data.display_name,
        bio: data.bio,
        avatar_bytes: data.avatar_bytes,
        avatar_content_type: data.avatar_content_type,
        banner_bytes: data.banner_bytes,
        banner_content_type: data.banner_content_type,
        also_known_as: data.also_known_as,
    };
    if let Err(e) = update_profile::execute(&state.app_ctx.deps.users.update_profile, cmd).await {
        tracing::error!("update_profile error: {:?}", e);
        return axum::response::Redirect::to(&format!(
            "/settings/profile?error={}",
            super::helpers::encode_error(&e.to_string())
        ))
        .into_response();
    }

    let fields: Vec<domain::models::ProfileField> = (0..4)
        .filter_map(|i| {
            data.profile_field_names
                .get(&i)
                .map(|name| domain::models::ProfileField {
                    name: name.clone(),
                    value: data
                        .profile_field_values
                        .get(&i)
                        .cloned()
                        .unwrap_or_default(),
                })
        })
        .collect();

    let fields_cmd = application::users::commands::UpdateProfileFieldsCommand {
        user_id: user_id.value(),
        fields,
    };
    if let Err(e) =
        update_profile_fields::execute(&state.app_ctx.deps.users.update_profile_fields, fields_cmd)
            .await
    {
        tracing::error!("update_profile_fields error: {:?}", e);
        return axum::response::Redirect::to(&format!(
            "/settings/profile?error={}",
            super::helpers::encode_error(&e.to_string())
        ))
        .into_response();
    }

    axum::response::Redirect::to("/settings/profile?saved=1").into_response()
}
