use crate::extractors::{Body, PathId, Upload};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::{DeleteAccountRequest, ProfileResponse, UserSummaryDto, UsersResponse};
use application::users::{
    get_users, queries::GetUsersQuery, request_account_deletion, update_profile,
    update_profile_fields,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use domain::value_objects::UserId;

#[utoipa::path(
    get, path = "/api/v1/profile",
    responses(
        (status = 200, body = ProfileResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "User not found", body = api_types::ErrorResponse),
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
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 500, description = "Internal server error", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Upload(multipart): Upload,
) -> impl IntoResponse {
    let data = crate::support::parse_profile_multipart(multipart).await;

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
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

#[utoipa::path(
    put, path = "/api/v1/profile/fields",
    request_body = api_types::UpdateProfileFieldsRequest,
    responses(
        (status = 204, description = "Profile fields updated"),
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 500, description = "Internal server error", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_profile_fields_handler(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Body(body): Body<serde_json::Value>,
) -> impl IntoResponse {
    let raw_fields = match body.get("fields").and_then(|f| f.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            return crate::errors::refuse(
                StatusCode::BAD_REQUEST,
                crate::errors::INVALID_INPUT,
                "body must contain a fields array",
            );
        }
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
        Err(e) => crate::errors::ApiError(e).into_response(),
    }
}

#[utoipa::path(
    get, path = "/api/v1/users",
    responses(
        (status = 200, body = UsersResponse),
        (status = 429, description = "Rate limited", body = api_types::ErrorResponse),
    ),
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

async fn delete_account(
    state: &AppState,
    actor: UserId,
    target: UserId,
    password: &str,
) -> Result<StatusCode, ApiError> {
    request_account_deletion::execute(
        &state.app_ctx.deps.users.request_account_deletion,
        actor,
        target,
        password,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete, path = "/api/v1/users/me",
    request_body = DeleteAccountRequest,
    responses(
        (status = 204, description = "Account deleted"),
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized or incorrect password", body = api_types::ErrorResponse),
        (status = 409, description = "The last admin cannot be deleted", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_me(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Body(req): Body<DeleteAccountRequest>,
) -> Result<StatusCode, ApiError> {
    delete_account(&state, user_id.clone(), user_id, &req.password).await
}

#[utoipa::path(
    delete, path = "/api/v1/users/{id}",
    params(("id" = uuid::Uuid, Path, description = "User ID")),
    request_body = DeleteAccountRequest,
    responses(
        (status = 204, description = "Account deleted"),
        (status = 400, description = "Invalid input", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized or incorrect password", body = api_types::ErrorResponse),
        (status = 403, description = "Admin only", body = api_types::ErrorResponse),
        (status = 404, description = "User not found", body = api_types::ErrorResponse),
        (status = 409, description = "The last admin cannot be deleted", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    AuthenticatedUser(actor): AuthenticatedUser,
    PathId(target): PathId<uuid::Uuid>,
    Body(req): Body<DeleteAccountRequest>,
) -> Result<StatusCode, ApiError> {
    delete_account(&state, actor, UserId::from_uuid(target), &req.password).await
}
