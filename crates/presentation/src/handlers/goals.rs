use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::{
    CreateGoalRequest, GoalDto, GoalsResponse, UpdateGoalRequest, UpdateUserSettingsRequest,
    UserSettingsDto,
};

// ── Shared mapper ────────────────────────────────────────────────────────────

pub fn goal_with_progress_to_dto(g: &domain::models::GoalWithProgress) -> GoalDto {
    GoalDto {
        year: g.goal.year(),
        target_count: g.goal.target_count(),
        current_count: g.current_count,
        percentage: g.percentage(),
        is_complete: g.is_complete(),
        goal_type: g.goal.goal_type().as_str().to_string(),
    }
}

// ── Goals API ────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/goals",
    responses(
        (status = 200, body = GoalsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_goals(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<GoalsResponse>, ApiError> {
    let goals = application::goals::list::execute(
        state.app_ctx.repos.goal.clone(),
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<CreateGoalRequest>,
) -> Result<Json<GoalDto>, ApiError> {
    let g = application::goals::create::execute(
        state.app_ctx.repos.goal.clone(),
        state.app_ctx.services.event_publisher.clone(),
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Goal not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(year): Path<u16>,
    Json(req): Json<UpdateGoalRequest>,
) -> Result<Json<GoalDto>, ApiError> {
    let g = application::goals::update::execute(
        state.app_ctx.repos.goal.clone(),
        state.app_ctx.services.event_publisher.clone(),
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Goal not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_goal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(year): Path<u16>,
) -> Result<StatusCode, ApiError> {
    application::goals::delete::execute(
        state.app_ctx.repos.goal.clone(),
        state.app_ctx.services.event_publisher.clone(),
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_user_goals(
    State(state): State<AppState>,
    AuthenticatedUser(_viewer): AuthenticatedUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<GoalsResponse>, ApiError> {
    let goals = application::goals::list::execute(
        state.app_ctx.repos.goal.clone(),
        application::goals::queries::ListGoalsQuery { user_id },
    )
    .await?;
    Ok(Json(GoalsResponse {
        goals: goals.iter().map(goal_with_progress_to_dto).collect(),
    }))
}

// ── User Settings ────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/settings",
    responses(
        (status = 200, body = UserSettingsDto),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserSettingsDto>, ApiError> {
    let settings =
        application::users::get_settings::execute(&state.app_ctx, user.0.value()).await?;
    Ok(Json(UserSettingsDto {
        federate_goals: settings.federate_goals(),
    }))
}

#[utoipa::path(
    put, path = "/api/v1/settings",
    request_body = UpdateUserSettingsRequest,
    responses(
        (status = 204, description = "Settings updated"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(req): Json<UpdateUserSettingsRequest>,
) -> Result<StatusCode, ApiError> {
    application::users::update_settings::execute(
        &state.app_ctx,
        application::users::update_settings::UpdateUserSettingsCommand {
            user_id: user.0.value(),
            federate_goals: req.federate_goals,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
