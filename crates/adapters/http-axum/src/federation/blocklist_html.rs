use crate::html::forms::{ActorUrlForm, BlockDomainForm, RemoveDomainForm};
use crate::html::helpers::build_page_context;
use crate::html::templates::{BlockedActorsTemplate, BlockedDomainsTemplate};
use crate::{
    extractors::RequiredCookieUser, html::csrf::CsrfToken, html::render::render_page,
    state::AppState,
};
use axum::{
    Form,
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

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
            let entries: Vec<crate::html::templates::BlockedDomainEntry> = domains
                .into_iter()
                .map(|d| crate::html::templates::BlockedDomainEntry {
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
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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

pub async fn post_remove_blocked_domain(
    crate::extractors::AdminUser(_): crate::extractors::AdminUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<RemoveDomainForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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
            let entries: Vec<crate::html::templates::BlockedActorEntry> = blocked
                .into_iter()
                .map(|a| crate::html::templates::BlockedActorEntry {
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

pub async fn post_block_actor_html(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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

pub async fn post_unblock_actor(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<ActorUrlForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
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
