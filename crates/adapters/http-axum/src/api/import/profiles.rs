use crate::extractors::{Body, PathId};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::SaveProfileRequest;
use application::import::{
    commands::{DeleteImportProfileCommand, SaveImportProfileCommand},
    delete_profile as delete_import_profile, list_profiles as list_import_profiles,
    save_profile as save_import_profile,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use domain::errors::DomainError;
use domain::value_objects::ImportSessionId;

#[utoipa::path(
    get, path = "/api/v1/import/profiles",
    responses(
        (status = 200, description = "List of saved import profiles", body = inline(Vec<serde_json::Value>)),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_get_profiles(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> Result<impl IntoResponse, ApiError> {
    let profiles =
        list_import_profiles::execute(&state.app_ctx.deps.import.list_profiles, &user_id).await?;
    Ok(axum::Json(
        profiles
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.value().to_string(),
                    "name": p.name,
                    "created_at": p.created_at.to_string(),
                })
            })
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/import/profiles",
    request_body = SaveProfileRequest,
    responses(
        (status = 200, description = "Profile saved", body = inline(serde_json::Value)),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 422, description = "Session has no mapping yet", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_post_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Body(body): Body<SaveProfileRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session_id = body
        .session_id
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
        .map_err(|_| DomainError::ValidationError("invalid session id".into()))?;
    let id = save_import_profile::execute(
        &state.app_ctx.deps.import.save_profile,
        SaveImportProfileCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            name: body.name,
        },
    )
    .await?;
    Ok(axum::Json(
        serde_json::json!({"id": id.value().to_string()}),
    ))
}

#[utoipa::path(
    delete, path = "/api/v1/import/profiles/{id}",
    params(("id" = String, Path, description = "Import profile UUID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 404, description = "Profile not found", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn api_delete_profile(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    PathId(profile_id_str): PathId<String>,
) -> Result<impl IntoResponse, ApiError> {
    let profile_id = profile_id_str
        .parse::<uuid::Uuid>()
        .map_err(|_| DomainError::ValidationError("invalid profile id".into()))?;
    delete_import_profile::execute(
        &state.app_ctx.deps.import.delete_profile,
        DeleteImportProfileCommand {
            user_id: user_id.value(),
            profile_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
