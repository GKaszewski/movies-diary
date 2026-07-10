use api_types::HtmlPageContext;
use domain::value_objects::UserId;

use crate::state::AppState;

pub(crate) fn encode_error(msg: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(msg, NON_ALPHANUMERIC).to_string()
}

pub(crate) fn build_export_response(
    format_str: &str,
    user_id: uuid::Uuid,
    state: &AppState,
) -> axum::response::Response {
    use axum::{body::Body, http::StatusCode, response::IntoResponse};
    use futures::StreamExt;

    use application::diary::{export_diary as export_diary_uc, queries::ExportQuery};
    use domain::models::ExportFormat;

    let format = match format_str {
        "csv" => ExportFormat::Csv,
        "json" => ExportFormat::Json,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let (content_type, filename) = match &format {
        ExportFormat::Csv => ("text/csv; charset=utf-8", "diary.csv"),
        ExportFormat::Json => ("application/json", "diary.json"),
    };
    let query = ExportQuery { user_id, format };
    let stream = export_diary_uc::execute(
        &state.app_ctx.repos.diary,
        &state.app_ctx.services.diary_exporter,
        query,
    );
    let stream = stream.map(|r| {
        if let Err(ref e) = r {
            tracing::error!("diary export stream error: {e}");
        }
        r
    });
    (
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, content_type.to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

pub(crate) struct ProfileFormData {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_bytes: Option<Vec<u8>>,
    pub avatar_content_type: Option<String>,
    pub banner_bytes: Option<Vec<u8>>,
    pub banner_content_type: Option<String>,
    pub also_known_as: Option<String>,
    pub profile_field_names: std::collections::HashMap<usize, String>,
    pub profile_field_values: std::collections::HashMap<usize, String>,
}

pub(crate) async fn parse_profile_multipart(
    mut multipart: axum::extract::Multipart,
) -> ProfileFormData {
    let mut data = ProfileFormData {
        display_name: None,
        bio: None,
        avatar_bytes: None,
        avatar_content_type: None,
        banner_bytes: None,
        banner_content_type: None,
        also_known_as: None,
        profile_field_names: std::collections::HashMap::new(),
        profile_field_values: std::collections::HashMap::new(),
    };

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "display_name" => {
                if let Ok(text) = field.text().await {
                    data.display_name = Some(text).filter(|s| !s.is_empty());
                }
            }
            "bio" => {
                if let Ok(text) = field.text().await {
                    data.bio = Some(text);
                }
            }
            "also_known_as" => {
                if let Ok(text) = field.text().await {
                    data.also_known_as = Some(text).filter(|s| !s.is_empty());
                }
            }
            "avatar" => {
                let ct = field.content_type().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await
                    && !bytes.is_empty()
                {
                    data.avatar_bytes = Some(bytes.to_vec());
                    data.avatar_content_type = ct;
                }
            }
            "banner" => {
                let ct = field.content_type().map(|s| s.to_string());
                if let Ok(bytes) = field.bytes().await
                    && !bytes.is_empty()
                {
                    data.banner_bytes = Some(bytes.to_vec());
                    data.banner_content_type = ct;
                }
            }
            n if n.starts_with("field_name_") => {
                if let Ok(idx) = n["field_name_".len()..].parse::<usize>()
                    && let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    data.profile_field_names.insert(idx, text);
                }
            }
            n if n.starts_with("field_value_") => {
                if let Ok(idx) = n["field_value_".len()..].parse::<usize>()
                    && let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    data.profile_field_values.insert(idx, text);
                }
            }
            _ => {}
        }
    }

    data
}

pub(crate) async fn build_page_context(
    state: &AppState,
    user_id: Option<UserId>,
    csrf_token: String,
) -> HtmlPageContext {
    let uuid = user_id.as_ref().map(|u| u.value());
    let (user_email, is_admin) = if let Some(ref id) = user_id {
        let user = state.app_ctx.repos.user.find_by_id(id).await.ok().flatten();
        let email = user.as_ref().map(|u| u.email().value().to_string());
        let admin = user
            .as_ref()
            .map(|u| matches!(u.role(), domain::models::UserRole::Admin))
            .unwrap_or(false);
        (email, admin)
    } else {
        (None, false)
    };
    HtmlPageContext {
        user_email,
        user_id: uuid,
        is_admin,
        register_enabled: state.app_ctx.config.allow_registration,
        rss_url: "/feed.rss".to_string(),
        page_title: "Movies Diary".to_string(),
        canonical_url: state.app_ctx.config.base_url.clone(),
        csrf_token,
        page_rss_url: None,
    }
}
