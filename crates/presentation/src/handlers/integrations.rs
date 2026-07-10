use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

use application::integrations::{
    commands::{
        ConfirmWatchEventsCommand, DismissWatchEventsCommand, GenerateWebhookTokenCommand,
        RevokeWebhookTokenCommand, WatchEventConfirmation,
    },
    confirm as confirm_watch_events, dismiss as dismiss_watch_events,
    generate_token as generate_webhook_token, get_queue as get_watch_queue,
    get_tokens as get_webhook_tokens,
    queries::{GetWatchQueueQuery, GetWebhookTokensQuery},
    revoke_token as revoke_webhook_token,
};

use crate::{
    csrf::CsrfToken, extractors::RequiredCookieUser, forms::ErrorQuery, render::render_page,
    state::AppState,
};
use template_askama::{IntegrationsTemplate, WatchQueueTemplate};

use super::helpers::{build_page_context, encode_error};

// ── HTML ─────────────────────────────────────────────────────────────────────

pub async fn get_integrations_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<crate::forms::IntegrationsQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Integrations — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/settings/integrations", state.app_ctx.config.base_url);

    let query = GetWebhookTokensQuery {
        user_id: user_id.value(),
    };
    let tokens = get_webhook_tokens::execute(state.app_ctx.repos.webhook_token.clone(), query)
        .await
        .unwrap_or_default();

    let token_views: Vec<template_askama::WebhookTokenView> = tokens
        .iter()
        .map(crate::mappers::integrations::webhook_token_view)
        .collect();

    let webhook_base_url = state.app_ctx.config.base_url.clone();
    render_page(IntegrationsTemplate {
        ctx: &ctx,
        tokens: &token_views,
        webhook_base_url: &webhook_base_url,
        new_token: params.token.as_deref(),
    })
    .into_response()
}

pub async fn post_generate_token(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<crate::forms::GenerateTokenForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let provider = match form.provider.parse::<domain::models::WatchEventSource>() {
        Ok(p) => p,
        Err(_) => return Redirect::to("/settings/integrations").into_response(),
    };

    let cmd = GenerateWebhookTokenCommand {
        user_id: user_id.value(),
        provider,
        label: form.label.filter(|l| !l.trim().is_empty()),
    };

    match generate_webhook_token::execute(state.app_ctx.repos.webhook_token.clone(), cmd).await {
        Ok(result) => {
            let encoded = percent_encoding::utf8_percent_encode(
                &result.token_plaintext,
                percent_encoding::NON_ALPHANUMERIC,
            );
            Redirect::to(&format!("/settings/integrations?token={encoded}")).into_response()
        }
        Err(e) => {
            tracing::error!("generate token failed: {:?}", e);
            Redirect::to("/settings/integrations").into_response()
        }
    }
}

pub async fn post_revoke_token(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(token_id): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<crate::forms::RevokeTokenForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = RevokeWebhookTokenCommand {
        user_id: user_id.value(),
        token_id,
    };
    if let Err(e) =
        revoke_webhook_token::execute(state.app_ctx.repos.webhook_token.clone(), cmd).await
    {
        tracing::error!("revoke token failed: {:?}", e);
    }

    Redirect::to("/settings/integrations").into_response()
}

// ── Watch Queue ──────────────────────────────────────────────────────────────

pub async fn get_watch_queue_page(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<ErrorQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Watch Queue — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/watch-queue", state.app_ctx.config.base_url);

    let query = GetWatchQueueQuery {
        user_id: user_id.value(),
    };
    let events = get_watch_queue::execute(state.app_ctx.repos.watch_event_query.clone(), query)
        .await
        .unwrap_or_default();

    let entries: Vec<template_askama::WatchQueueDisplayEntry> = events
        .iter()
        .map(crate::mappers::integrations::watch_queue_entry)
        .collect();

    render_page(WatchQueueTemplate {
        ctx: &ctx,
        entries: &entries,
        error: params.error.as_deref(),
    })
    .into_response()
}

pub async fn post_confirm_single(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<crate::forms::ConfirmWatchForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = ConfirmWatchEventsCommand {
        user_id: user_id.value(),
        confirmations: vec![WatchEventConfirmation {
            watch_event_id: event_id,
            rating: form.rating,
            comment: form.comment.filter(|c| !c.trim().is_empty()),
        }],
    };

    match confirm_watch_events::execute(
        state.app_ctx.repos.watch_event_command.clone(),
        state.app_ctx.repos.watch_event_query.clone(),
        state.app_ctx.services.review_logger.clone(),
        cmd,
    )
    .await
    {
        Ok(_) => Redirect::to("/watch-queue").into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/watch-queue?error={msg}")).into_response()
        }
    }
}

pub async fn post_dismiss_single(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<crate::forms::DismissWatchForm>,
) -> impl IntoResponse {
    if crate::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = DismissWatchEventsCommand {
        user_id: user_id.value(),
        event_ids: vec![event_id],
    };

    match dismiss_watch_events::execute(
        state.app_ctx.repos.watch_event_command.clone(),
        state.app_ctx.repos.watch_event_query.clone(),
        cmd,
    )
    .await
    {
        Ok(_) => Redirect::to("/watch-queue").into_response(),
        Err(e) => {
            let msg = encode_error(&e.to_string());
            Redirect::to(&format!("/watch-queue?error={msg}")).into_response()
        }
    }
}
