use axum::{
    Form, Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

use crate::{
    csrf::CsrfToken,
    errors::ApiError,
    extractors::{AuthenticatedUser, RequiredCookieUser},
    forms::{FollowForm, FollowerActionForm, UnfollowForm},
    render::render_page,
    state::AppState,
};
#[cfg(feature = "federation")]
use crate::{
    extractors::AdminApiUser,
    forms::{ActorUrlForm, BlockDomainForm, RemoveDomainForm},
};
use api_types::{
    ActorListResponse, ActorUrlRequest, FollowRelationResponse, FollowRequest,
    PendingCountResponse, RemoteActorDto,
};
#[cfg(feature = "federation")]
use api_types::{AddBlockedDomainRequest, BlockedActorResponse, BlockedDomainResponse};
use domain::value_objects::{FollowTarget, InstanceIdentity, SocialActor, SocialIdentity};
#[cfg(feature = "federation")]
use template_askama::{BlockedActorsTemplate, BlockedDomainsTemplate};
use template_askama::{FollowersTemplate, FollowingTemplate, RemoteActorData};

use super::helpers::{build_page_context, encode_domain_error};

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

#[cfg(feature = "federation")]
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

fn social_actor_to_template(actor: SocialActor, instance: &InstanceIdentity) -> RemoteActorData {
    RemoteActorData {
        url: instance.actor_url_of(&actor.identity),
        handle: actor.handle,
        display_name: actor.display_name,
        avatar_url: actor.avatar_url,
    }
}

// ── API ──────────────────────────────────────────────────────────────────────

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/admin/blocked-domains",
    responses(
        (status = 200, body = Vec<BlockedDomainResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
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

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/admin/blocked-domains",
    request_body = AddBlockedDomainRequest,
    responses(
        (status = 201, description = "Domain blocked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    axum::Json(body): axum::Json<AddBlockedDomainRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app_ctx
        .ap_blocklist
        .add_blocked_domain(&body.domain, body.reason.as_deref())
        .await?;
    Ok(StatusCode::CREATED)
}

#[cfg(feature = "federation")]
#[utoipa::path(
    delete, path = "/api/v1/admin/blocked-domains/{domain}",
    params(("domain" = String, Path, description = "Domain to unblock")),
    responses(
        (status = 204, description = "Domain unblocked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_blocked_domain_admin(
    State(state): State<AppState>,
    _admin: AdminApiUser,
    axum::extract::Path(domain): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app_ctx
        .ap_blocklist
        .remove_blocked_domain(&domain)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/block",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor blocked"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn block_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::Json(body): axum::Json<ActorUrlRequest>,
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

#[cfg(feature = "federation")]
#[utoipa::path(
    post, path = "/api/v1/social/unblock",
    request_body = ActorUrlRequest,
    responses(
        (status = 204, description = "Actor unblocked"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unblock_actor_api(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::Json(body): axum::Json<ActorUrlRequest>,
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

#[cfg(feature = "federation")]
#[utoipa::path(
    get, path = "/api/v1/social/blocked",
    responses(
        (status = 200, body = Vec<BlockedActorResponse>),
        (status = 401, description = "Unauthorized"),
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

#[utoipa::path(
    get, path = "/api/v1/social/following",
    responses(
        (status = 200, body = ActorListResponse),
        (status = 401, description = "Unauthorized"),
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
        (status = 401, description = "Unauthorized"),
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
    Path(user_id): Path<Uuid>,
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
    Path(user_id): Path<Uuid>,
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn follow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<FollowRequest>,
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unfollow(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn accept_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn reject_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_follower(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<ActorUrlRequest>,
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
        (status = 401, description = "Unauthorized"),
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
        (status = 401, description = "Unauthorized"),
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
        (status = 401, description = "Unauthorized"),
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_relationship(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(q): Query<RelationshipQuery>,
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

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn follow_user(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let redirect_base = form
        .redirect_after
        .as_deref()
        .filter(|u| u.starts_with('/') && !u.starts_with("//"))
        .unwrap_or(&format!("/users/{}", profile_user_uuid))
        .to_string();

    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Follow {
            follower_id: user_id.value(),
            target: FollowTarget::Handle(form.handle),
        },
    )
    .await
    {
        Ok(()) => Redirect::to(&redirect_base).into_response(),
        Err(e) => {
            tracing::error!("follow error: {:?}", e);
            let msg = encode_domain_error(&e);
            let sep = if redirect_base.contains('?') {
                '&'
            } else {
                '?'
            };
            Redirect::to(&format!("{}{}error={}", redirect_base, sep, msg)).into_response()
        }
    }
}

pub async fn unfollow_user(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<UnfollowForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Unfollow {
            follower_id: user_id.value(),
            target: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(()) => {
            Redirect::to(&format!("/users/{}/following-list", profile_user_uuid)).into_response()
        }
        Err(e) => {
            let msg = encode_domain_error(&e);
            Redirect::to(&format!(
                "/users/{}/following-list?error={}",
                profile_user_uuid, msg
            ))
            .into_response()
        }
    }
}

pub async fn accept_follower_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::AcceptFollow {
            owner_id: user_id.value(),
            requester: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_domain_error(&e);
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

pub async fn reject_follower_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::RejectFollow {
            owner_id: user_id.value(),
            requester: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_domain_error(&e);
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_followers_collection(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
        let page = params.get("page").and_then(|p| p.parse::<u32>().ok());
        return match state
            .app_ctx
            .ap_document
            .followers_collection_json(user_id, page)
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
            Err(e) => {
                tracing::error!("followers_collection_json error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }
    axum::response::Redirect::to(&format!("/users/{}/followers-list", user_id)).into_response()
}

#[cfg(feature = "federation")]
pub async fn get_following_collection(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/activity+json") || accept.contains("application/ld+json") {
        let page = params.get("page").and_then(|p| p.parse::<u32>().ok());
        return match state
            .app_ctx
            .ap_document
            .following_collection_json(user_id, page)
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
            Err(e) => {
                tracing::error!("following_collection_json error: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }
    axum::response::Redirect::to(&format!("/users/{}/following-list", user_id)).into_response()
}

pub async fn get_following_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Query(params): Query<crate::forms::ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Following — Movies Diary".to_string();
    ctx.canonical_url = format!(
        "{}/users/{}/following-list",
        state.app_ctx.config.base_url, profile_user_uuid
    );
    let instance = state.app_ctx.instance.clone();
    match application::social::get_following::execute(
        &state.app_ctx.deps.social.query,
        user_id.value(),
    )
    .await
    {
        Ok(following) => {
            let actors: Vec<RemoteActorData> = following
                .into_iter()
                .map(|a| social_actor_to_template(a, &instance))
                .collect();
            let pending_actors: Vec<RemoteActorData> =
                match application::social::get_pending_following::execute(
                    &state.app_ctx.deps.social.query,
                    user_id.value(),
                )
                .await
                {
                    Ok(pending) => pending
                        .into_iter()
                        .map(|a| social_actor_to_template(a, &instance))
                        .collect(),
                    Err(e) => {
                        tracing::error!("get_pending_following error: {:?}", e);
                        Vec::new()
                    }
                };
            render_page(FollowingTemplate {
                ctx,
                user_id: profile_user_uuid,
                actors,
                pending_actors,
                error: params.error,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("get_following error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load following list",
            )
                .into_response()
        }
    }
}

pub async fn get_followers_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Query(params): Query<crate::forms::ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Followers — Movies Diary".to_string();
    ctx.canonical_url = format!(
        "{}/users/{}/followers-list",
        state.app_ctx.config.base_url, profile_user_uuid
    );
    let instance = state.app_ctx.instance.clone();
    match application::social::get_followers::execute(
        &state.app_ctx.deps.social.query,
        user_id.value(),
    )
    .await
    {
        Ok(followers) => {
            let actors: Vec<RemoteActorData> = followers
                .into_iter()
                .map(|a| social_actor_to_template(a, &instance))
                .collect();
            render_page(FollowersTemplate {
                ctx,
                user_id: profile_user_uuid,
                actors,
                error: params.error,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("get_followers error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load followers list",
            )
                .into_response()
        }
    }
}

pub async fn remove_follower_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<FollowerActionForm>,
) -> impl IntoResponse {
    if user_id.value() != profile_user_uuid {
        return StatusCode::FORBIDDEN.into_response();
    }
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::RemoveFollower {
            owner_id: user_id.value(),
            follower: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(_) => {
            Redirect::to(&format!("/users/{}/followers-list", profile_user_uuid)).into_response()
        }
        Err(e) => {
            let msg = encode_domain_error(&e);
            Redirect::to(&format!(
                "/users/{}/followers-list?error={}",
                profile_user_uuid, msg
            ))
            .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_blocked_domains_page(
    crate::extractors::AdminUser(user_id): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id), csrf.0).await;
    ctx.page_title = "Blocked Domains — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/admin/blocked-domains", state.app_ctx.config.base_url);
    match state.app_ctx.ap_blocklist.get_blocked_domains().await {
        Ok(domains) => {
            let entries: Vec<template_askama::BlockedDomainEntry> = domains
                .into_iter()
                .map(|d| template_askama::BlockedDomainEntry {
                    domain: d.domain,
                    reason: d.reason,
                    blocked_at: d.blocked_at,
                })
                .collect();
            render_page(BlockedDomainsTemplate {
                ctx: &ctx,
                domains: &entries,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("get_blocked_domains error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load blocked domains",
            )
                .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_blocked_domain(
    crate::extractors::AdminUser(_): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<BlockDomainForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let reason = form.reason.as_deref().filter(|s| !s.trim().is_empty());
    match state
        .app_ctx
        .ap_blocklist
        .add_blocked_domain(&form.domain, reason)
        .await
    {
        Ok(()) => Redirect::to("/admin/blocked-domains").into_response(),
        Err(e) => {
            tracing::error!("add_blocked_domain error: {:?}", e);
            Redirect::to("/admin/blocked-domains").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_remove_blocked_domain(
    crate::extractors::AdminUser(_): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<RemoveDomainForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .app_ctx
        .ap_blocklist
        .remove_blocked_domain(&form.domain)
        .await
    {
        Ok(()) => Redirect::to("/admin/blocked-domains").into_response(),
        Err(e) => {
            tracing::error!("remove_blocked_domain error: {:?}", e);
            Redirect::to("/admin/blocked-domains").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn get_blocked_actors_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Blocked Users — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/social/blocked", state.app_ctx.config.base_url);
    let instance = state.app_ctx.instance.clone();
    match application::social::get_blocked::execute(
        &state.app_ctx.deps.social.query,
        user_id.value(),
    )
    .await
    {
        Ok(blocked) => {
            let entries: Vec<template_askama::BlockedActorEntry> = blocked
                .into_iter()
                .map(|a| template_askama::BlockedActorEntry {
                    url: instance.actor_url_of(&a.identity),
                    handle: a.handle,
                    display_name: a.display_name,
                    avatar_url: a.avatar_url,
                })
                .collect();
            render_page(BlockedActorsTemplate {
                ctx: &ctx,
                actors: &entries,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("get_blocked_actors error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load blocked users",
            )
                .into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_block_actor_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Block {
            blocker_id: user_id.value(),
            target: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("block_actor html error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}

#[cfg(feature = "federation")]
pub async fn post_unblock_actor(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let instance = state.app_ctx.instance.clone();
    match application::social::execute::execute_command(
        &state.app_ctx.deps.social.command,
        application::social::commands::SocialCmd::Unblock {
            blocker_id: user_id.value(),
            target: instance.identify(&form.actor_url),
        },
    )
    .await
    {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("unblock_actor error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::value_objects::{InstanceIdentity, SocialActor, SocialIdentity, UserId};

    fn local_actor(uid: &UserId) -> SocialActor {
        SocialActor {
            identity: SocialIdentity::Local(uid.clone()),
            handle: "@gabriel@md.example".into(),
            display_name: None,
            avatar_url: None,
        }
    }

    #[test]
    fn local_actor_dto_url_round_trips_back_to_the_same_identity() {
        let instance = InstanceIdentity::new("https://md.example");
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());

        let dto = social_actor_to_dto(local_actor(&uid), &instance);

        assert!(!dto.url.starts_with("local:"), "url was {}", dto.url);
        assert_eq!(
            instance.identify(&dto.url),
            SocialIdentity::Local(uid),
            "a local actor's dto url must parse back as Local"
        );
    }

    #[cfg(feature = "federation")]
    #[test]
    fn local_blocked_dto_url_round_trips_back_to_the_same_identity() {
        let instance = InstanceIdentity::new("https://md.example");
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());

        let dto = social_actor_to_blocked_dto(local_actor(&uid), &instance);

        assert!(!dto.url.starts_with("local:"), "url was {}", dto.url);
        assert_eq!(instance.identify(&dto.url), SocialIdentity::Local(uid));
    }

    #[test]
    fn local_template_url_round_trips_back_to_the_same_identity() {
        let instance = InstanceIdentity::new("https://md.example");
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());

        let data = social_actor_to_template(local_actor(&uid), &instance);

        assert!(!data.url.starts_with("local:"), "url was {}", data.url);
        assert_eq!(instance.identify(&data.url), SocialIdentity::Local(uid));
    }

    #[test]
    fn remote_actor_dto_url_is_passed_through_unchanged() {
        let instance = InstanceIdentity::new("https://md.example");
        let actor = SocialActor {
            identity: SocialIdentity::Remote {
                actor_url: "https://other.example/users/bob".into(),
            },
            handle: "@bob@other.example".into(),
            display_name: None,
            avatar_url: None,
        };

        let dto = social_actor_to_dto(actor, &instance);

        assert_eq!(dto.url, "https://other.example/users/bob");
    }

    #[test]
    fn local_actor_dto_carries_user_id_and_avatar() {
        let instance = InstanceIdentity::new("https://md.example");
        let uid = UserId::from_uuid(uuid::Uuid::new_v4());
        let mut actor = local_actor(&uid);
        actor.avatar_url = Some("https://md.example/images/a.webp".into());

        let dto = social_actor_to_dto(actor, &instance);

        assert_eq!(
            dto.user_id,
            Some(uid.value()),
            "local actors must carry user_id for internal links"
        );
        assert_eq!(
            dto.avatar_url.as_deref(),
            Some("https://md.example/images/a.webp")
        );
    }

    #[test]
    fn remote_actor_dto_has_no_user_id() {
        let instance = InstanceIdentity::new("https://md.example");
        let actor = SocialActor {
            identity: SocialIdentity::Remote {
                actor_url: "https://other.example/users/bob".into(),
            },
            handle: "@bob@other.example".into(),
            display_name: None,
            avatar_url: None,
        };
        assert_eq!(social_actor_to_dto(actor, &instance).user_id, None);
    }
}
