use crate::html::helpers::encode_domain_error;
use crate::html::render::render_page;
use crate::html::templates::{ImportProfileView, ImportUploadTemplate};
use crate::{extractors::RequiredCookieUser, html::csrf::CsrfToken, state::AppState};
use application::import::{
    commands::{CreateImportSessionCommand, DeleteImportProfileCommand},
    create_session as create_import_session, delete_profile as delete_import_profile,
    list_profiles as list_import_profiles,
};
use axum::{
    Extension, Form,
    extract::{Multipart, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use domain::models::FileFormat;
use serde::Deserialize;
use std::collections::HashMap;

pub async fn get_import_page(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let ctx = crate::html::helpers::build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    let profiles =
        list_import_profiles::execute(&state.app_ctx.deps.import.list_profiles, &user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|p| ImportProfileView {
                id: p.id.value().to_string(),
                name: p.name,
            })
            .collect::<Vec<_>>();
    render_page(ImportUploadTemplate {
        ctx: &ctx,
        profiles: &profiles,
        error: None,
    })
}

pub async fn post_upload(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut format_str = "csv".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Ok(bytes) = field.bytes().await {
                    file_bytes = Some(bytes.to_vec());
                }
            }
            Some("format") => {
                if let Ok(text) = field.text().await {
                    format_str = text;
                }
            }
            _ => {}
        }
    }

    let bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return Redirect::to("/import?error=no+file+provided").into_response(),
    };

    let format = match format_str.as_str() {
        "json" => FileFormat::Json,
        "xlsx" => FileFormat::Xlsx,
        _ => FileFormat::Csv,
    };

    match create_import_session::execute(
        &state.app_ctx.deps.import.create_session,
        CreateImportSessionCommand {
            user_id: user_id.value(),
            bytes,
            format,
        },
    )
    .await
    {
        Ok(r) => Redirect::to(&format!("/import/{}/mapping", r.session_id.value())).into_response(),
        Err(e) => {
            Redirect::to(&format!("/import?error={}", encode_domain_error(&e))).into_response()
        }
    }
}

pub async fn get_import_done(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Extension(csrf): Extension<CsrfToken>,
    axum::extract::Query(params): axum::extract::Query<ImportDoneParams>,
) -> impl IntoResponse {
    let _ctx =
        crate::html::helpers::build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    let html = format!(
        r#"<!doctype html><html><body>
        <h1>Import Complete</h1>
        <p>Imported: {}</p>
        <p>Skipped duplicates: {}</p>
        <p>Failed: {}</p>
        <a href="/users/{}">Go to My Profile</a>
        </body></html>"#,
        params.imported.unwrap_or(0),
        params.skipped.unwrap_or(0),
        params.failed.unwrap_or(0),
        user_id.value(),
    );
    Html(html)
}

#[derive(Deserialize)]
pub struct ImportDoneParams {
    pub imported: Option<usize>,
    pub skipped: Option<usize>,
    pub failed: Option<usize>,
}

pub async fn post_delete_profile(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(profile_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let csrf_token = form.get("_csrf").map(|s| s.as_str()).unwrap_or("");
    if crate::html::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    if let Ok(profile_id) = profile_id_str.parse::<uuid::Uuid>() {
        let _ = delete_import_profile::execute(
            &state.app_ctx.deps.import.delete_profile,
            DeleteImportProfileCommand {
                user_id: user_id.value(),
                profile_id,
            },
        )
        .await;
    }
    Redirect::to("/import").into_response()
}
