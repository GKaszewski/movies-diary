use crate::extractors::{Body, Params, PathId};
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::{
    ActorListResponse, ActorUrlRequest, FollowRelationResponse, FollowRequest,
    PendingCountResponse, RemoteActorDto,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use domain::value_objects::{FollowTarget, InstanceIdentity, SocialActor, SocialIdentity};
use uuid::Uuid;

fn social_actor_to_dto(actor: SocialActor, instance: &InstanceIdentity) -> RemoteActorDto {
    RemoteActorDto {
        url: instance.actor_url_of(&actor.identity),
        user_id: match &actor.identity {
            SocialIdentity::Local(uid) => Some(uid.value()),
            SocialIdentity::Remote { .. } => None,
        },
        handle: actor.handle,
        display_name: actor.display_name,
        avatar_url: actor.avatar_url,
    }
}

#[utoipa::path(
    get, path = "/api/v1/social/following",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_following(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities = application::social::get_following::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
    )
    .await?;
    Ok(Json(ActorListResponse {
        actors: identities
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/social/followers",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_followers(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities = application::social::get_followers::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
    )
    .await?;
    Ok(Json(ActorListResponse {
        actors: identities
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

pub async fn get_user_following(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    PathId(user_id): PathId<Uuid>,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities =
        application::social::get_following::execute(&state.app_ctx.deps.social.query, user_id)
            .await?;
    Ok(Json(ActorListResponse {
        actors: identities
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

pub async fn get_user_followers(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    PathId(user_id): PathId<Uuid>,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities =
        application::social::get_followers::execute(&state.app_ctx.deps.social.query, user_id)
            .await?;
    Ok(Json(ActorListResponse {
        actors: identities
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/social/follow",
    request_body = FollowRequest,
    responses(
        (status = 200, description = "Follow request sent"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn follow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<FollowRequest>,
) -> Result<impl IntoResponse, ApiError> {
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Follow {
            follower_id: user.0.value(),
            target: FollowTarget::Handle(body.handle),
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post, path = "/api/v1/social/unfollow",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Unfollowed"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unfollow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Unfollow {
            follower_id: user.0.value(),
            target: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post, path = "/api/v1/social/followers/accept",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower accepted"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn accept_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::AcceptFollow {
            owner_id: user.0.value(),
            requester: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post, path = "/api/v1/social/followers/reject",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower rejected"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn reject_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::RejectFollow {
            owner_id: user.0.value(),
            requester: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post, path = "/api/v1/social/followers/remove",
    request_body = ActorUrlRequest,
    responses(
        (status = 200, description = "Follower removed"),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Body(body): Body<ActorUrlRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let instance = state.app_ctx.instance.clone();
    application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::RemoveFollower {
            owner_id: user.0.value(),
            follower: instance.identify(&body.actor_url),
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get, path = "/api/v1/social/followers/pending",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_followers(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let identities = application::social::get_pending_followers::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
    )
    .await?;
    Ok(Json(ActorListResponse {
        actors: identities
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/social/followers/pending/count",
    responses(
        (status = 200, body = PendingCountResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_follower_count(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<PendingCountResponse>, ApiError> {
    let count = application::social::count_pending_followers::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
    )
    .await?;
    Ok(Json(PendingCountResponse { count }))
}

#[utoipa::path(
    get, path = "/api/v1/social/following/pending",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_following(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ActorListResponse>, ApiError> {
    let instance = state.app_ctx.instance.clone();
    let actors = application::social::get_pending_following::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
    )
    .await?;
    Ok(Json(ActorListResponse {
        actors: actors
            .into_iter()
            .map(|a| social_actor_to_dto(a, &instance))
            .collect(),
    }))
}

#[derive(serde::Deserialize)]
pub struct RelationshipQuery {
    pub actor_url: String,
}

#[utoipa::path(
    get, path = "/api/v1/social/relationship",
    params(("actor_url" = String, Query, description = "Canonical actor URL of the target")),
    responses(
        (status = 200, body = FollowRelationResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_relationship(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Params(q): Params<RelationshipQuery>,
) -> Result<Json<FollowRelationResponse>, ApiError> {
    let target = state.app_ctx.instance.identify(&q.actor_url);
    let rel = application::social::get_relation::execute(
        &state.app_ctx.deps.social.query,
        user.0.value(),
        target,
    )
    .await?;
    Ok(Json(FollowRelationResponse {
        following: rel.following.into(),
        followed_by: rel.followed_by.into(),
    }))
}

#[cfg(test)]
mod tests;
