use chrono::NaiveDateTime;
use uuid::Uuid;

/// One denormalized row per Review — everything the insight analyzer needs,
/// nothing it doesn't (ADR-0010). Lighter than `WrapUpMovieRow` on purpose:
/// WrapUp's row is a report-era artifact with poster paths, budgets and cast
/// that Insights must not inherit.
#[derive(Clone, Debug)]
pub struct WatchRow {
    pub movie_id: Uuid,
    pub rating: u8,
    pub watched_at: NaiveDateTime,
    pub watch_medium: Option<String>,
    pub genres: Vec<String>,
}

/// A single conservative, baseline-relative statement about a user's viewing
/// behavior (see CONTEXT.md "Insight"). Every variant carries the numbers the
/// SPA needs to render the claim plus the `sample_count` behind it, so the UI
/// can show why a category is empty without another round trip.
///
/// Ordering contract: `build_insights` returns insights grouped by category in
/// a fixed order (rating lift, seasonality, medium timing, rhythm), strongest
/// first within each category, at most `MAX_PER_CATEGORY` per category.
#[derive(Clone, Debug, PartialEq)]
pub enum Insight {
    /// The genre's average rating sits `delta` stars away from the user's own
    /// average. First-time watches only — a rewatch is chosen because it is
    /// already liked, so counting it would conflate choice with genre.
    RatingLift {
        genre: String,
        genre_avg: f64,
        user_avg: f64,
        delta: f64,
        sample_count: u32,
    },
    /// The genre's share of watches in one month of the year is `multiplier`
    /// times the user's own share of watches in that month.
    Seasonality {
        genre: String,
        /// 1–12
        month: u32,
        multiplier: f64,
        sample_count: u32,
    },
    /// The medium's weekend share of watches is `multiplier` times the user's
    /// overall weekend share.
    MediumTiming {
        medium: String,
        multiplier: f64,
        weekend_share: f64,
        baseline_share: f64,
        sample_count: u32,
    },
    /// When watching actually happens: the 3-hour window and the weekdays that
    /// carry the most real-timestamped watches.
    Rhythm {
        /// 0–23, start of the peak 3-hour window.
        peak_hour_start: u32,
        /// 0–23, exclusive end of the peak window (`start + 3`, mod 24).
        peak_hour_end: u32,
        /// Chrono weekday numbers, strongest first: Mon=1 … Sun=7.
        top_days: Vec<u32>,
        sample_count: u32,
    },
}
