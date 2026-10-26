use crate::html::forms::DeleteAccountForm;
use crate::html::helpers::build_page_context;
use crate::html::templates::{
    ProfileSettingsTemplate, RemoteActorDisplay, UserSummaryView, UsersTemplate,
};
use crate::{
    extractors::{OptionalCookieUser, RequiredCookieUser},
    html::csrf::CsrfToken,
    html::render::render_page,
    state::AppState,
};
use application::users::{
    get_profile_settings as get_profile_settings_uc, resolve_username_to_id, update_profile,
    update_profile_fields,
};
use axum::{
    Form,
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use domain::{
    errors::DomainError,
    value_objects::{UserId, Username},
};
use uuid::Uuid;

pub async fn get_users_list(
    OptionalCookieUser(user_id): OptionalCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, user_id, csrf.0).await;
    ctx.page_title = "Members — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/users", state.app_ctx.config.base_url);

    match application::users::get_users::execute(
        &state.app_ctx.deps.users.get_users_list,
        application::users::queries::GetUsersQuery,
    )
    .await
    {
        Ok(result) => {
            let users: Vec<UserSummaryView> = result
                .users
                .iter()
                .map(crate::html::mappers::users::user_summary_view)
                .collect();
            let remote_actors: Vec<RemoteActorDisplay> = result
                .remote_actors
                .iter()
                .map(crate::html::mappers::users::remote_actor_display)
                .collect();
            render_page(UsersTemplate {
                users,
                ctx: &ctx,
                remote_actors,
            })
            .into_response()
        }
        Err(e) => crate::errors::domain_error_response(e),
    }
}

pub async fn get_user_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    let uname = match Username::new(username) {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    match resolve_username_to_id::execute(&state.app_ctx.deps.users.resolve_username, &uname).await
    {
        Ok(Some(uid)) => {
            axum::response::Redirect::permanent(&format!("/users/{}", uid.value())).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(serde::Deserialize, Default)]
pub struct SavedQuery {
    pub saved: Option<String>,
    pub error: Option<String>,
}

pub async fn get_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Query(params): Query<SavedQuery>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let mut ctx = build_page_context(&state, Some(user_id.clone()), csrf.0).await;
    ctx.page_title = "Profile Settings — Movies Diary".to_string();
    ctx.canonical_url = format!("{}/settings/profile", state.app_ctx.config.base_url);

    let settings = match get_profile_settings_uc::execute(
        &state.app_ctx.deps.users.get_profile_settings,
        user_id.value(),
    )
    .await
    {
        Ok(s) => s,
        Err(DomainError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return crate::errors::domain_error_response(e),
    };

    let saved = params.saved.as_deref() == Some("1");

    render_page(ProfileSettingsTemplate {
        ctx: &ctx,
        bio: settings.bio.as_deref(),
        avatar_url: settings.avatar_url.as_deref(),
        banner_url: settings.banner_url.as_deref(),
        also_known_as: settings.also_known_as.as_deref(),
        profile_fields: &settings.fields,
        saved,
        error: params.error.as_deref(),
        embed_url: format!(
            "{}/users/{}?embed=true",
            state.app_ctx.config.base_url,
            user_id.value()
        ),
    })
    .into_response()
}

pub async fn post_profile_settings(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    multipart: Multipart,
) -> impl IntoResponse {
    let data = crate::support::parse_profile_multipart(multipart).await;

    let cmd = application::users::commands::UpdateProfileCommand {
        user_id: user_id.value(),
        display_name: data.display_name,
        bio: data.bio,
        avatar_bytes: data.avatar_bytes,
        avatar_content_type: data.avatar_content_type,
        banner_bytes: data.banner_bytes,
        banner_content_type: data.banner_content_type,
        also_known_as: data.also_known_as,
    };
    if let Err(e) = update_profile::execute(&state.app_ctx.deps.users.update_profile, cmd).await {
        tracing::error!("update_profile error: {:?}", e);
        return axum::response::Redirect::to(&format!(
            "/settings/profile?error={}",
            crate::html::helpers::encode_domain_error(&e)
        ))
        .into_response();
    }

    let fields: Vec<domain::models::ProfileField> = (0..4)
        .filter_map(|i| {
            data.profile_field_names
                .get(&i)
                .map(|name| domain::models::ProfileField {
                    name: name.clone(),
                    value: data
                        .profile_field_values
                        .get(&i)
                        .cloned()
                        .unwrap_or_default(),
                })
        })
        .collect();

    let fields_cmd = application::users::commands::UpdateProfileFieldsCommand {
        user_id: user_id.value(),
        fields,
    };
    if let Err(e) =
        update_profile_fields::execute(&state.app_ctx.deps.users.update_profile_fields, fields_cmd)
            .await
    {
        tracing::error!("update_profile_fields error: {:?}", e);
        return axum::response::Redirect::to(&format!(
            "/settings/profile?error={}",
            crate::html::helpers::encode_domain_error(&e)
        ))
        .into_response();
    }

    axum::response::Redirect::to("/settings/profile?saved=1").into_response()
}

async fn delete_account(
    state: &AppState,
    actor: UserId,
    target: UserId,
    password: &str,
) -> Result<(), DomainError> {
    application::users::request_account_deletion::execute(
        &state.app_ctx.deps.users.request_account_deletion,
        actor,
        target,
        password,
    )
    .await
}

pub async fn post_delete_own_account(
    RequiredCookieUser(user_id): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Form(form): Form<DeleteAccountForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match delete_account(&state, user_id.clone(), user_id, &form.password).await {
        Ok(()) => super::auth::get_logout(State(state)).await.into_response(),
        Err(e) => axum::response::Redirect::to(&format!(
            "/settings/profile?error={}",
            crate::html::helpers::encode_domain_error(&e)
        ))
        .into_response(),
    }
}

pub async fn post_delete_user(
    RequiredCookieUser(actor): RequiredCookieUser,
    State(state): State<AppState>,
    Extension(csrf): Extension<CsrfToken>,
    Path(target): Path<Uuid>,
    Form(form): Form<DeleteAccountForm>,
) -> impl IntoResponse {
    if crate::html::csrf::mismatch(&csrf, &form.csrf_token) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let target_id = UserId::from_uuid(target);
    let own = actor == target_id;
    match delete_account(&state, actor, target_id, &form.password).await {
        Ok(()) if own => super::auth::get_logout(State(state)).await.into_response(),
        Ok(()) => axum::response::Redirect::to("/users").into_response(),
        Err(e @ DomainError::Forbidden(_)) => crate::errors::domain_error_response(e),
        Err(e) => axum::response::Redirect::to(&format!(
            "/users/{target}?error={}",
            crate::html::helpers::encode_domain_error(&e)
        ))
        .into_response(),
    }
}
