use crate::extractors::{Params, PathId};
use crate::{extractors::AuthenticatedUser, state::AppState};
use api_types::mappers::goal_with_progress_to_dto;
use api_types::{
    ActorStatDto, DiaryResponse, DirectorStatDto, GenreStatDto, MonthActivityDto, MonthlyRatingDto,
    UserProfileQueryParams, UserProfileResponse, UserStatsDto, UserTrendsDto, WatchMediumStatDto,
};
use application::users::{
    get_federated_profile, get_federated_profile_stats, get_local_profile,
    queries::GetUserProfileQuery,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use domain::errors::DomainError;
use std::str::FromStr;
use uuid::Uuid;

#[utoipa::path(
    get, path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID"),
        UserProfileQueryParams,
    ),
    responses(
        (status = 200, body = UserProfileResponse),
        (status = 404, description = "User not found", body = api_types::ErrorResponse),
    )
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    AuthenticatedUser(viewer_id): AuthenticatedUser,
    PathId(user_id): PathId<Uuid>,
    Params(params): Params<UserProfileQueryParams>,
) -> impl IntoResponse {
    let view_str = params.view.as_deref().unwrap_or("recent");
    let profile_view = match application::users::queries::ProfileView::from_str(view_str) {
        Ok(v) => v,
        Err(_) => {
            return crate::errors::refuse(
                StatusCode::BAD_REQUEST,
                crate::errors::INVALID_INPUT,
                "invalid profile view",
            );
        }
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
            return crate::errors::refuse(
                StatusCode::NOT_FOUND,
                crate::errors::NOT_FOUND,
                "not found",
            );
        }
        Err(e) => return crate::errors::ApiError(e).into_response(),
    };

    let view_data = if let Some(p) = profile.entries {
        Some(api_types::ProfileViewData::Entries {
            entries: DiaryResponse {
                items: p
                    .items
                    .iter()
                    .map(api_types::mappers::entry_to_dto)
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
                        .map(api_types::mappers::entry_to_dto)
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
        is_service_account: domain::value_objects::UserId::from_uuid(user_id).is_service_actor(),
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
        Err(e) => return crate::errors::ApiError(e).into_response(),
    };

    let view_data = if let Some(p) = profile.entries {
        Some(api_types::ProfileViewData::Entries {
            entries: DiaryResponse {
                items: p
                    .items
                    .iter()
                    .map(api_types::mappers::entry_to_dto)
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
        is_service_account: false,
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
