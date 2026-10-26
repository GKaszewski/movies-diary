use crate::extractors::{Body, PathId};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::mappers::goal_with_progress_to_dto;
use api_types::{
    CreateGoalRequest, GoalDto, GoalsResponse, UpdateGoalRequest, UpdateUserSettingsRequest,
    UserSettingsDto,
};
use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

#[utoipa::path(
    get, path = "/api/v1/goals",
    responses(
        (status = 200, body = GoalsResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_goals(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<GoalsResponse>, ApiError> {
    let goals = application::goals::list::execute(
        &state.app_ctx.deps.goals.query,
        application::goals::queries::ListGoalsQuery {
            user_id: user.0.value(),
        },
    )
    .await?;
    Ok(Json(GoalsResponse {
        goals: goals.iter().map(goal_with_progress_to_dto).collect(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/goals",
    request_body = CreateGoalRequest,
    responses(
        (status = 200, body = GoalDto),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(req): Body<CreateGoalRequest>,
) -> Result<Json<GoalDto>, ApiError> {
    let g = application::goals::create::execute(
        &state.app_ctx.deps.goals.command,
        application::goals::commands::CreateGoalCommand {
            user_id: user.0.value(),
            year: req.year,
            target_count: req.target_count,
        },
    )
    .await?;
    Ok(Json(goal_with_progress_to_dto(&g)))
}

#[utoipa::path(
    put, path = "/api/v1/goals/{year}",
    request_body = UpdateGoalRequest,
    responses(
        (status = 200, body = GoalDto),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Goal not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    PathId(year): PathId<u16>,
    Body(req): Body<UpdateGoalRequest>,
) -> Result<Json<GoalDto>, ApiError> {
    let g = application::goals::update::execute(
        &state.app_ctx.deps.goals.command,
        application::goals::commands::UpdateGoalCommand {
            user_id: user.0.value(),
            year,
            target_count: req.target_count,
        },
    )
    .await?;
    Ok(Json(goal_with_progress_to_dto(&g)))
}

#[utoipa::path(
    delete, path = "/api/v1/goals/{year}",
    responses(
        (status = 204, description = "Goal deleted"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Goal not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    PathId(year): PathId<u16>,
) -> Result<StatusCode, ApiError> {
    application::goals::delete::execute(
        &state.app_ctx.deps.goals.command,
        application::goals::commands::DeleteGoalCommand {
            user_id: user.0.value(),
            year,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/api/v1/users/{id}/goals",
    responses(
        (status = 200, body = GoalsResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_user_goals(
    State(state): State<AppState>,
    AuthenticatedUser(_viewer): AuthenticatedUser,
    PathId(user_id): PathId<Uuid>,
) -> Result<Json<GoalsResponse>, ApiError> {
    let goals = application::goals::list::execute(
        &state.app_ctx.deps.goals.query,
        application::goals::queries::ListGoalsQuery { user_id },
    )
    .await?;
    Ok(Json(GoalsResponse {
        goals: goals.iter().map(goal_with_progress_to_dto).collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/settings",
    responses(
        (status = 200, body = UserSettingsDto),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserSettingsDto>, ApiError> {
    let settings = application::users::get_settings::execute(
        &state.app_ctx.deps.users.get_settings,
        user.0.value(),
    )
    .await?;
    Ok(Json(UserSettingsDto {
        federate_goals: settings.federate_goals(),
        federate_reviews: settings.federate_reviews(),
        federate_watchlist: settings.federate_watchlist(),
    }))
}

#[utoipa::path(
    put, path = "/api/v1/settings",
    request_body = UpdateUserSettingsRequest,
    responses(
        (status = 204, description = "Settings updated"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(req): Body<UpdateUserSettingsRequest>,
) -> Result<StatusCode, ApiError> {
    application::users::update_settings::execute(
        &state.app_ctx.deps.users.update_settings,
        application::users::update_settings::UpdateUserSettingsCommand {
            user_id: user.0.value(),
            federate_goals: req.federate_goals,
            federate_reviews: req.federate_reviews,
            federate_watchlist: req.federate_watchlist,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
