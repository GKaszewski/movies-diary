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
    forms::{
        ActorUrlForm, BlockDomainForm, FollowForm, FollowerActionForm, RemoveDomainForm,
        UnfollowForm,
    },
    render::render_page,
    state::AppState,
};
use api_types::{
    ActorListResponse, ActorUrlRequest, AddBlockedDomainRequest, BlockedActorResponse,
    BlockedDomainResponse, FollowRequest, RemoteActorDto,
};
use template_askama::{
    BlockedActorsTemplate, BlockedDomainsTemplate, FollowersTemplate, FollowingTemplate,
    RemoteActorData,
};

use super::helpers::build_page_context;

fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

fn ap_err(e: anyhow::Error) -> impl IntoResponse {
    tracing::error!("ActivityPub error: {:?}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

fn ap_to_domain(e: anyhow::Error) -> domain::errors::DomainError {
    tracing::error!("ActivityPub error: {:?}", e);
    domain::errors::DomainError::InfrastructureError(e.to_string())
}

// ── API ──────────────────────────────────────────────────────────────────────

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
    _admin: crate::extractors::AdminUser,
) -> impl IntoResponse {
    match state.ap_service.get_blocked_domains().await {
        Ok(domains) => {
            let response: Vec<BlockedDomainResponse> = domains
                .into_iter()
                .map(|d| BlockedDomainResponse {
                    domain: d.domain,
                    reason: d.reason,
                    blocked_at: d.blocked_at,
                })
                .collect();
            axum::Json(response).into_response()
        }
        Err(e) => ap_err(e).into_response(),
    }
}

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
    _admin: crate::extractors::AdminUser,
    axum::Json(body): axum::Json<AddBlockedDomainRequest>,
) -> impl IntoResponse {
    match state
        .ap_service
        .add_blocked_domain(&body.domain, body.reason.as_deref())
        .await
    {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

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
    _admin: crate::extractors::AdminUser,
    axum::extract::Path(domain): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.ap_service.remove_blocked_domain(&domain).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

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
) -> impl IntoResponse {
    match state
        .ap_service
        .block_actor(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

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
) -> impl IntoResponse {
    match state
        .ap_service
        .unblock_actor(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

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
) -> impl IntoResponse {
    match state.ap_service.get_blocked_actors(user.0.value()).await {
        Ok(actors) => {
            let response: Vec<BlockedActorResponse> = actors
                .into_iter()
                .map(|a| BlockedActorResponse {
                    url: a.url,
                    handle: a.handle,
                    display_name: a.display_name,
                    avatar_url: a.avatar_url,
                })
                .collect();
            axum::Json(response).into_response()
        }
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state.ap_service.get_following(user.0.value()).await {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state
        .ap_service
        .get_accepted_followers(user.0.value())
        .await
    {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

pub async fn get_user_following(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ActorListResponse>, ApiError> {
    let actors = state
        .ap_service
        .get_following(user_id)
        .await
        .map_err(ap_to_domain)?;
    Ok(Json(ActorListResponse {
        actors: actors
            .into_iter()
            .map(|a| RemoteActorDto {
                handle: a.handle,
                display_name: a.display_name,
                url: a.url,
            })
            .collect(),
    }))
}

pub async fn get_user_followers(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ActorListResponse>, ApiError> {
    let actors = state
        .ap_service
        .get_accepted_followers(user_id)
        .await
        .map_err(ap_to_domain)?;
    Ok(Json(ActorListResponse {
        actors: actors
            .into_iter()
            .map(|a| RemoteActorDto {
                handle: a.handle,
                display_name: a.display_name,
                url: a.url,
            })
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
) -> impl IntoResponse {
    match state.ap_service.follow(user.0.value(), &body.handle).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state
        .ap_service
        .unfollow(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state
        .ap_service
        .accept_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state
        .ap_service
        .reject_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state
        .ap_service
        .remove_follower(user.0.value(), &body.actor_url)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => ap_err(e).into_response(),
    }
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
) -> impl IntoResponse {
    match state.ap_service.get_pending_followers(user.0.value()).await {
        Ok(actors) => Json(ActorListResponse {
            actors: actors
                .into_iter()
                .map(|a| RemoteActorDto {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => ap_err(e).into_response(),
    }
}

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn follow_remote_user(
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

    match state.ap_service.follow(user_id.value(), &form.handle).await {
        Ok(()) => Redirect::to(&redirect_base).into_response(),
        Err(e) => {
            tracing::error!("follow error: {:?}", e);
            let msg = encode_error(&e.to_string());
            let sep = if redirect_base.contains('?') {
                '&'
            } else {
                '?'
            };
            Redirect::to(&format!("{}{}error={}", redirect_base, sep, msg)).into_response()
        }
    }
}

pub async fn unfollow_remote_user(
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
    match state
        .ap_service
        .unfollow(user_id.value(), &form.actor_url)
        .await
    {
        Ok(()) => {
            Redirect::to(&format!("/users/{}/following-list", profile_user_uuid)).into_response()
        }
        Err(e) => {
            let msg = encode_error(&e.to_string());
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
    match state
        .ap_service
        .accept_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
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
    match state
        .ap_service
        .reject_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => Redirect::to(&format!("/users/{}", profile_user_uuid)).into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/users/{}?error={}", profile_user_uuid, msg)).into_response()
        }
    }
}

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
            .ap_service
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
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        };
    }
    axum::response::Redirect::to(&format!("/users/{}/followers-list", user_id)).into_response()
}

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
            .ap_service
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
            Err(_) => StatusCode::NOT_FOUND.into_response(),
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
    match state.ap_service.get_following(user_id.value()).await {
        Ok(following) => {
            let actors: Vec<RemoteActorData> = following
                .into_iter()
                .map(|a| RemoteActorData {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                    avatar_url: a.avatar_url.clone(),
                })
                .collect();
            render_page(FollowingTemplate {
                ctx,
                user_id: profile_user_uuid,
                actors,
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
    match state
        .ap_service
        .get_accepted_followers(user_id.value())
        .await
    {
        Ok(followers) => {
            let actors: Vec<RemoteActorData> = followers
                .into_iter()
                .map(|a| RemoteActorData {
                    handle: a.handle,
                    display_name: a.display_name,
                    url: a.url,
                    avatar_url: a.avatar_url.clone(),
                })
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
    match state
        .ap_service
        .remove_follower(user_id.value(), &form.actor_url)
        .await
    {
        Ok(_) => {
            Redirect::to(&format!("/users/{}/followers-list", profile_user_uuid)).into_response()
        }
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!(
                "/users/{}/followers-list?error={}",
                profile_user_uuid, msg
            ))
            .into_response()
        }
    }
}

pub async fn get_blocked_domains_page(
    crate::extractors::AdminUser(user_id): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id), csrf.0).await;
    ctx.page_title = "Blocked Domains — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/admin/blocked-domains", state.app_ctx.config.base_url);
    match state.ap_service.get_blocked_domains().await {
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
        .ap_service
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

pub async fn post_remove_blocked_domain(
    crate::extractors::AdminUser(_): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<RemoveDomainForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state.ap_service.remove_blocked_domain(&form.domain).await {
        Ok(()) => Redirect::to("/admin/blocked-domains").into_response(),
        Err(e) => {
            tracing::error!("remove_blocked_domain error: {:?}", e);
            Redirect::to("/admin/blocked-domains").into_response()
        }
    }
}

pub async fn get_blocked_actors_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Blocked Users — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/social/blocked", state.app_ctx.config.base_url);
    match state.ap_service.get_blocked_actors(user_id.value()).await {
        Ok(actors) => {
            let entries: Vec<template_askama::BlockedActorEntry> = actors
                .into_iter()
                .map(|a| template_askama::BlockedActorEntry {
                    url: a.url,
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

pub async fn post_block_actor_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .block_actor(user_id.value(), &form.actor_url)
        .await
    {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("block_actor html error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}

pub async fn post_unblock_actor(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .ap_service
        .unblock_actor(user_id.value(), &form.actor_url)
        .await
    {
        Ok(()) => Redirect::to("/social/blocked").into_response(),
        Err(e) => {
            tracing::error!("unblock_actor error: {:?}", e);
            Redirect::to("/social/blocked").into_response()
        }
    }
}
