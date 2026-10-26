use crate::extractors::AdminApiUser;
use crate::extractors::{Body, PathId};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::ActorUrlRequest;
use api_types::{AddBlockedDomainRequest, BlockedActorResponse, BlockedDomainResponse};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use domain::value_objects::{InstanceIdentity, SocialActor};

fn social_actor_to_blocked_dto(
    actor: SocialActor,
    instance: &InstanceIdentity,
) -> BlockedActorResponse {
    BlockedActorResponse {
        url: instance.actor_url_of(&actor.identity),
        handle: actor.handle,
        display_name: actor.display_name,
        avatar_url: actor.avatar_url,
    }
}

#[utoipa::path(
    get, path = "/api/v1/admin/blocked-domains",
    responses(
        (status = 200, body = Vec<BlockedDomainResponse>),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_blocked_domains_admin(
    State(state): State<AppState>,
    _admin: AdminApiUser,
) -> Result<Json<Vec<BlockedDomainResponse>>, ApiError> {
    let domains = state.app_ctx.ap_blocklist.get_blocked_domains().await?;
    Ok(Json(
        domains
            .into_iter()
            .map(|d| BlockedDomainResponse {
                domain: d.domain,
                reason: d.reason,
                blocked_at: d.blocked_at,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/admin/blocked-domains",
    request_body = AddBlockedDomainRequest,
    responses(
        (status = 201, description = "Domain blocked"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    Body(body): Body<AddBlockedDomainRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app_ctx
        .ap_blocklist
        .add_blocked_domain(&body.domain, body.reason.as_deref())
        .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete, path = "/api/v1/admin/blocked-domains/{domain}",
    params(("domain" = String, Path, description = "Domain to unblock")),
    responses(
        (status = 204, description = "Domain unblocked"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
        (status = 403, description = "Forbidden - admin only", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    PathId(domain): PathId<String>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app_ctx
        .ap_blocklist
        .remove_blocked_domain(&domain)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/api/v1/social/block",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor blocked"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn block_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Block {
            blocker_id: user.0.value(),
            target: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/api/v1/social/unblock",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor unblocked"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unblock_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Unblock {
            blocker_id: user.0.value(),
            target: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/api/v1/social/blocked",
    responses(
        (status = 200, body = Vec<BlockedActorResponse>),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_blocked_actors_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<BlockedActorResponse>>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities =
        application::social::get_blocked::execute(&state.app_ctx.deps.social.query, user.0.value())
            .await?;
    Ok(Json(
        identities
            .into_iter()
            .map(|a| social_actor_to_blocked_dto(a, &instance))
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
