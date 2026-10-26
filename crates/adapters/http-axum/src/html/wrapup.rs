use crate::{
    extractors::OptionalCookieUser, html::csrf::CsrfToken, html::render::render_page,
    state::AppState,
};
use application::wrapup::get_ready_report;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use domain::errors::DomainError;
use domain::models::wrapup::{WrapUpReport, WrapUpScope};
use uuid::Uuid;

fn format_watch_time(minutes: u32) -> String {
    let h = minutes / 60;
    let m = minutes % 60;
    if h > 0 && m > 0 {
        format!("{}h {}m", h, m)
    } else if h > 0 {
        format!("{}h", h)
    } else {
        format!("{}m", m)
    }
}

fn render_wrapup(
    report: &WrapUpReport,
    year: i32,
    ctx: &api_types::HtmlPageContext,
) -> axum::response::Response {
    let rating_max = report
        .rating_distribution
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let rating_pcts: [f64; 5] =
        std::array::from_fn(|i| report.rating_distribution[i] as f64 / rating_max as f64 * 100.0);
    let genre_max = report
        .top_genres
        .first()
        .map(|g| g.count)
        .unwrap_or(1)
        .max(1);
    let genre_pcts: Vec<f64> = report
        .top_genres
        .iter()
        .take(8)
        .map(|g| g.count as f64 / genre_max as f64 * 100.0)
        .collect();
    let tmpl = crate::html::templates::WrapUpPageTemplate {
        ctx,
        report,
        year_label: year.to_string(),
        watch_time_display: format_watch_time(report.total_watch_time_minutes),
        rating_max,
        genre_max,
        rating_pcts,
        genre_pcts,
    };
    render_page(tmpl)
}

pub async fn get_user_wrapup_html(
    OptionalCookieUser(viewer): OptionalCookieUser,
    State(state): State<AppState>,
    Path((user_id, year)): Path<(Uuid, i32)>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let scope = WrapUpScope::User(user_id);
    let report =
        match get_ready_report::execute(&state.app_ctx.deps.wrapup.get_ready_report, scope, year)
            .await
        {
            Ok(report) => report,
            Err(DomainError::ValidationError(_)) => return StatusCode::BAD_REQUEST.into_response(),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        };

    let ctx = crate::html::helpers::build_page_context(&state, viewer, csrf.0).await;
    render_wrapup(&report, year, &ctx)
}

pub async fn get_global_wrapup_html(
    OptionalCookieUser(viewer): OptionalCookieUser,
    State(state): State<AppState>,
    Path(year): Path<i32>,
    Extension(csrf): Extension<CsrfToken>,
) -> impl IntoResponse {
    let report = match get_ready_report::execute(
        &state.app_ctx.deps.wrapup.get_ready_report,
        WrapUpScope::Global,
        year,
    )
    .await
    {
        Ok(report) => report,
        Err(DomainError::ValidationError(_)) => return StatusCode::BAD_REQUEST.into_response(),
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let ctx = crate::html::helpers::build_page_context(&state, viewer, csrf.0).await;
    render_wrapup(&report, year, &ctx)
}
