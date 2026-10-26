use crate::extractors::Params;
use crate::{errors::ApiError, extractors::AuthenticatedUser, state::AppState};
use api_types::insights::{InsightDto, InsightsQueryParams, InsightsResponse};
use application::insights::get as get_insights_uc;
use axum::{Json, extract::State};
use domain::{errors::DomainError, models::Insight};

fn to_dto(insight: Insight) -> InsightDto {
    match insight {
        Insight::RatingLift {
            genre,
            genre_avg,
            user_avg,
            delta,
            sample_count,
        } => InsightDto::RatingLift {
            genre,
            genre_avg,
            user_avg,
            delta,
            sample_count,
        },
        Insight::Seasonality {
            genre,
            month,
            multiplier,
            sample_count,
        } => InsightDto::Seasonality {
            genre,
            month,
            multiplier,
            sample_count,
        },
        Insight::MediumTiming {
            medium,
            multiplier,
            weekend_share,
            baseline_share,
            sample_count,
        } => InsightDto::MediumTiming {
            medium,
            multiplier,
            weekend_share,
            baseline_share,
            sample_count,
        },
        Insight::Rhythm {
            peak_hour_start,
            peak_hour_end,
            top_days,
            sample_count,
        } => InsightDto::Rhythm {
            peak_hour_start,
            peak_hour_end,
            top_days,
            sample_count,
        },
    }
}

/// Live insights for the authenticated user's own diary. Private surface:
/// habits (late-night watching, mediums) are more sensitive than the taste
/// statistics already public on profiles (ADR-0010).
#[utoipa::path(
    get, path = "/api/v1/insights",
    params(InsightsQueryParams),
    responses(
        (status = 200, body = InsightsResponse),
        (status = 400, description = "Unknown time zone", body = api_types::ErrorResponse),
        (status = 401, description = "Unauthorized", body = api_types::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_insights(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Params(params): Params<InsightsQueryParams>,
) -> Result<Json<InsightsResponse>, ApiError> {
    let tz: chrono_tz::Tz = match params.tz.as_deref() {
        None => chrono_tz::UTC,
        Some(name) => name.parse().map_err(|_| {
            ApiError(DomainError::ValidationError(format!(
                "unknown time zone: {name}"
            )))
        })?,
    };
    let insights = get_insights_uc::execute(&state.app_ctx.deps.insights.get, user.0, &tz).await?;
    Ok(Json(InsightsResponse {
        insights: insights.into_iter().map(to_dto).collect(),
    }))
}
