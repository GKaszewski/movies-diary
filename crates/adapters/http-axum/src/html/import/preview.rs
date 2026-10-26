use crate::html::helpers::encode_domain_error;
use crate::html::mappers::import::annotated_to_preview_row;
use crate::html::render::render_page;
use crate::html::templates::{ImportPreviewRow, ImportPreviewTemplate};
use crate::{extractors::RequiredCookieUser, html::csrf::CsrfToken, state::AppState};
use application::import::{
    commands::{ExecuteImportCommand, SaveImportProfileCommand},
    execute as execute_import, get_preview_stage, save_profile as save_import_profile,
};
use axum::{
    Extension, Form,
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use domain::value_objects::ImportSessionId;

pub async fn get_preview_page(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };
    let Ok(stage) = get_preview_stage::execute(
        &state.app_ctx.deps.import.get_preview_stage,
        session_id,
        user_id.value(),
    )
    .await
    else {
        return Redirect::to("/import").into_response();
    };

    let preview = match stage {
        get_preview_stage::PreviewStage::Ready(preview) => preview,
        get_preview_stage::PreviewStage::NotYetMapped => {
            return Redirect::to(&format!("/import/{}/mapping", session_id_str)).into_response();
        }
    };

    let rows: Vec<ImportPreviewRow> = preview
        .rows
        .iter()
        .enumerate()
        .map(|(i, a)| annotated_to_preview_row(i, a))
        .collect();

    let ctx = crate::html::helpers::build_page_context(&state, Some(user_id), csrf.0).await;
    render_page(ImportPreviewTemplate {
        ctx: &ctx,
        session_id: &session_id_str,
        columns: &preview.columns,
        rows: &rows,
    })
    .into_response()
}

pub async fn post_confirm(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form_entries): Form<Vec<(String, String)>>,
) -> impl IntoResponse {
    let csrf_token = form_entries
        .iter()
        .find(|(k, _)| k == "_csrf")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    if crate::html::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };

    // Save profile if name provided
    let profile_name = form_entries
        .iter()
        .find(|(k, _)| k == "profile_name")
        .map(|(_, v)| v.clone())
        .filter(|n| !n.trim().is_empty());
    if let Some(name) = profile_name {
        let _ = save_import_profile::execute(
            &state.app_ctx.deps.import.save_profile,
            SaveImportProfileCommand {
                user_id: user_id.value(),
                session_id: session_id.value(),
                name,
            },
        )
        .await;
    }

    // Collect all "confirmed" checkbox values
    let confirmed: Vec<usize> = form_entries
        .iter()
        .filter(|(k, _)| k == "confirmed")
        .filter_map(|(_, v)| v.parse::<usize>().ok())
        .collect();

    match execute_import::execute(
        &state.app_ctx.deps.import.execute_import,
        ExecuteImportCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            confirmed_indices: confirmed,
        },
    )
    .await
    {
        Ok(summary) => Redirect::to(&format!(
            "/import/done?imported={}&skipped={}&failed={}",
            summary.imported,
            summary.skipped_duplicates,
            summary.failed.len()
        ))
        .into_response(),
        Err(e) => {
            Redirect::to(&format!("/import?error={}", encode_domain_error(&e))).into_response()
        }
    }
}
