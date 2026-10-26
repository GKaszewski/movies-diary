use crate::html::helpers::encode_domain_error;
use crate::html::render::render_page;
use crate::html::templates::ImportMappingTemplate;
use crate::support::str_to_domain_field;
use crate::{extractors::RequiredCookieUser, html::csrf::CsrfToken, state::AppState};
use application::import::{
    apply_mapping as apply_import_mapping, commands::ApplyImportMappingCommand, get_mapping_stage,
};
use axum::{
    Extension, Form,
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use domain::models::{
    FieldMapping,
    import::{DomainField, Transform},
};
use domain::value_objects::ImportSessionId;
use std::collections::HashMap;

fn parse_mapping_form(form: &HashMap<String, String>) -> Vec<FieldMapping> {
    let mut mappings = Vec::new();
    let mut i = 0usize;
    loop {
        if i > 64 {
            break;
        }
        let col_key = format!("mapping_{i}_col");
        let Some(col) = form.get(&col_key).cloned() else {
            break;
        };
        let field_str = form
            .get(&format!("mapping_{i}_field"))
            .map(|s| s.as_str())
            .unwrap_or("");
        if let Some(domain_field) = str_to_domain_field(field_str) {
            let transform = if domain_field == DomainField::Rating {
                let scale: f64 = form
                    .get(&format!("mapping_{i}_scale"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1.0);
                Transform::RatingScale(scale)
            } else if domain_field == DomainField::WatchedAt {
                form.get(&format!("mapping_{i}_datefmt"))
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .map(Transform::DateFormat)
                    .unwrap_or(Transform::Identity)
            } else {
                Transform::Identity
            };
            mappings.push(FieldMapping {
                source_column: col,
                domain_field,
                transform,
            });
        }
        i += 1;
    }
    mappings
}

pub async fn get_mapping_page(
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
    let Ok(stage) = get_mapping_stage::execute(
        &state.app_ctx.deps.import.get_mapping_stage,
        session_id,
        user_id.value(),
    )
    .await
    else {
        return Redirect::to("/import").into_response();
    };

    let ctx = crate::html::helpers::build_page_context(&state, Some(user_id), csrf.0).await;
    let domain_fields: Vec<(&str, &str)> = vec![
        ("title", "Title"),
        ("release_year", "Release Year"),
        ("director", "Director"),
        ("rating", "Rating"),
        ("watched_at", "Watched At"),
        ("comment", "Comment"),
        ("external_metadata_id", "External ID"),
    ];
    render_page(ImportMappingTemplate {
        ctx: &ctx,
        session_id: &session_id_str,
        columns: &stage.columns,
        sample_rows: &stage.sample_rows,
        domain_fields: &domain_fields,
        error: None,
    })
    .into_response()
}

pub async fn post_mapping(
    State(state): State<AppState>,
    RequiredCookieUser(user_id): RequiredCookieUser,
    Path(session_id_str): Path<String>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let csrf_token = form.get("_csrf").map(|s| s.as_str()).unwrap_or("");
    if crate::html::csrf::mismatch(&csrf, csrf_token) {
        return Redirect::to("/import").into_response();
    }
    let Ok(session_id) = session_id_str
        .parse::<uuid::Uuid>()
        .map(ImportSessionId::from_uuid)
    else {
        return Redirect::to("/import").into_response();
    };
    let mappings = parse_mapping_form(&form);
    if mappings.is_empty() {
        return Redirect::to(&format!(
            "/import/{}/mapping?error=select+at+least+one+mapping",
            session_id_str
        ))
        .into_response();
    }
    match apply_import_mapping::execute(
        &state.app_ctx.deps.import.apply_mapping,
        ApplyImportMappingCommand {
            user_id: user_id.value(),
            session_id: session_id.value(),
            mappings,
        },
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/import/{}/preview", session_id_str)).into_response(),
        Err(e) => Redirect::to(&format!(
            "/import/{}/mapping?error={}",
            session_id_str,
            encode_domain_error(&e)
        ))
        .into_response(),
    }
}
