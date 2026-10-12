use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One conservative, baseline-relative statement about the user's viewing
/// behavior. Tagged by `kind` so the SPA can switch on it; every variant
/// carries `sample_count` so the UI can explain a thin sample.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InsightDto {
    /// `genre`'s average rating sits `delta` stars from the user's own
    /// average (first-time watches only).
    RatingLift {
        genre: String,
        genre_avg: f64,
        user_avg: f64,
        delta: f64,
        sample_count: u32,
    },
    /// `genre`'s share of watches in `month` (1–12) is `multiplier` times the
    /// user's own share of watches in that month.
    Seasonality {
        genre: String,
        month: u32,
        multiplier: f64,
        sample_count: u32,
    },
    /// `medium`'s weekend share of watches is `multiplier` times the user's
    /// overall weekend share.
    MediumTiming {
        medium: String,
        multiplier: f64,
        weekend_share: f64,
        baseline_share: f64,
        sample_count: u32,
    },
    /// Watching concentrates in a 3-hour window starting `peak_hour_start`
    /// (0–23) on `top_days` (Mon=1 … Sun=7).
    Rhythm {
        peak_hour_start: u32,
        peak_hour_end: u32,
        top_days: Vec<u32>,
        sample_count: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsResponse {
    pub insights: Vec<InsightDto>,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct InsightsQueryParams {
    /// IANA time zone (e.g. `Europe/Warsaw`) months, weekdays and hours are
    /// bucketed in. Defaults to UTC.
    pub tz: Option<String>,
}
