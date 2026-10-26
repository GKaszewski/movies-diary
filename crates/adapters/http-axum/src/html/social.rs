use crate::html::helpers::{build_page_context, encode_domain_error};
use crate::html::templates::{FollowersTemplate, FollowingTemplate, RemoteActorData};
use crate::{
    extractors::RequiredCookieUser,
    html::csrf::CsrfToken,
    html::forms::{FollowForm, FollowerActionForm, UnfollowForm},
    html::render::render_page,
    state::AppState,
};
use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use domain::value_objects::{FollowTarget, InstanceIdentity, SocialActor};
use uuid::Uuid;

fn social_actor_to_template(actor: SocialActor, instance: &InstanceIdentity) -> RemoteActorData {
    RemoteActorData {
        url: instance.actor_url_of(&actor.identity),
        handle: actor.handle,
        display_name: actor.display_name,
        avatar_url: actor.avatar_url,
    }
}

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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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

pub async fn get_following_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(profile_user_uuid): Path<Uuid>,
    Query(params): Query<crate::html::forms::ErrorQuery>,
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
    Query(params): Query<crate::html::forms::ErrorQuery>,
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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

#[cfg(test)]
mod tests;
