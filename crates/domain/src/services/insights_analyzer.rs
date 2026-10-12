//! Conservative insight extraction over a user's whole diary (ADR-0010).
//!
//! Every threshold lives here, not in adapters, so the honesty rules are
//! unit-testable without a database. An insight that cannot clear its
//! thresholds is silently absent — an empty page is the correct rendering of
//! a small or noisy diary, not a bug to fix by lowering thresholds.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike, Weekday};

use crate::models::{Insight, WatchRow};

// ── Honesty thresholds ───────────────────────────────────────────────────────

/// First-time ratings a genre needs before its average means anything.
const MIN_GENRE_RATINGS: usize = 5;
/// First-time ratings overall before the user's own average is a stable baseline.
const MIN_BASELINE_RATINGS: usize = 10;
/// Minimum distance from the user's average before a rating lift is worth saying.
const MIN_RATING_DELTA: f64 = 0.3;
/// Rows a genre needs before month-of-year distribution is more than noise.
const MIN_GENRE_SEASON_ROWS: usize = 15;
/// Minimum lift over the user's own share of watches in that month.
const MIN_SEASON_MULTIPLIER: f64 = 1.5;
/// Genre watches the peak month needs — a lift built on one or two rows in a
/// quiet month is noise, however large the ratio.
const MIN_SEASON_PEAK_ROWS: u32 = 4;
/// Rows a medium needs before its weekend share means anything.
const MIN_MEDIUM_ROWS: usize = 5;
/// Minimum lift over the user's own weekend share.
const MIN_MEDIUM_MULTIPLIER: f64 = 1.5;
/// More than this share of rows without a medium → the medium-timing category
/// would describe half a diary; skip it entirely.
const MAX_MISSING_MEDIUM_SHARE: f64 = 0.3;
/// Real-timestamped rows needed to say anything about rhythm.
const MIN_RHYTHM_ROWS: usize = 10;
/// More than this share of rows at exactly 00:00 → timestamps are import noise
/// (CSV imports carry no time); skip rhythm entirely.
const MAX_MIDNIGHT_SHARE: f64 = 0.5;
/// Insights shown per category — more than this and the page is a wall.
const MAX_PER_CATEGORY: usize = 3;
/// Width of the peak-hour window in the rhythm insight.
const RHYTHM_WINDOW_HOURS: u32 = 3;

/// `rows` carry `watched_at` in UTC as stored; `tz` is the viewer's zone, so
/// months, weekdays and hours are bucketed as the user lived them.
pub fn build_insights<Tz: TimeZone>(rows: &[WatchRow], tz: &Tz) -> Vec<Insight> {
    let mut insights = Vec::new();
    insights.extend(rating_lifts(rows));
    insights.extend(seasonalities(rows, tz));
    insights.extend(medium_timings(rows, tz));
    insights.extend(rhythm(rows, tz));
    insights
}

fn local<Tz: TimeZone>(tz: &Tz, utc: &NaiveDateTime) -> NaiveDateTime {
    tz.from_utc_datetime(utc).naive_local()
}

// ── Rating lift ──────────────────────────────────────────────────────────────

fn rating_lifts(rows: &[WatchRow]) -> Vec<Insight> {
    let first_time = first_time_rows(rows);
    if first_time.len() < MIN_BASELINE_RATINGS {
        return vec![];
    }
    let user_avg = avg_rating(&first_time);

    let mut by_genre: HashMap<&str, Vec<u8>> = HashMap::new();
    for row in first_time {
        for genre in &row.genres {
            by_genre.entry(genre).or_default().push(row.rating);
        }
    }

    let mut candidates: Vec<Insight> = by_genre
        .into_iter()
        .filter(|(_, ratings)| ratings.len() >= MIN_GENRE_RATINGS)
        .filter_map(|(genre, ratings)| {
            let genre_avg =
                ratings.iter().map(|&r| f64::from(r)).sum::<f64>() / ratings.len() as f64;
            let delta = round2(genre_avg - user_avg);
            if delta.abs() < MIN_RATING_DELTA {
                return None;
            }
            Some(Insight::RatingLift {
                genre: genre.to_string(),
                genre_avg: round2(genre_avg),
                user_avg: round2(user_avg),
                delta,
                sample_count: ratings.len() as u32,
            })
        })
        .collect();

    candidates.sort_by(strongest_first);
    candidates.truncate(MAX_PER_CATEGORY);
    candidates
}

/// Strongest first; ties broken by label ascending so output is deterministic.
fn strongest_first(a: &Insight, b: &Insight) -> std::cmp::Ordering {
    strength_of(b)
        .partial_cmp(&strength_of(a))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| label_of(a).cmp(label_of(b)))
}

/// The earliest review of each movie; later reviews of the same movie are
/// rewatches (the same definition `ReviewHistory` uses).
fn first_time_rows(rows: &[WatchRow]) -> Vec<&WatchRow> {
    let mut seen: HashMap<uuid::Uuid, &WatchRow> = HashMap::new();
    for row in rows {
        match seen.get(&row.movie_id) {
            Some(existing) if existing.watched_at <= row.watched_at => {}
            _ => {
                seen.insert(row.movie_id, row);
            }
        }
    }
    let mut first: Vec<&WatchRow> = seen.into_values().collect();
    first.sort_by_key(|r| r.watched_at);
    first
}

fn avg_rating(rows: &[&WatchRow]) -> f64 {
    rows.iter().map(|r| f64::from(r.rating)).sum::<f64>() / rows.len() as f64
}

// ── Seasonality ──────────────────────────────────────────────────────────────

fn seasonalities<Tz: TimeZone>(rows: &[WatchRow], tz: &Tz) -> Vec<Insight> {
    if rows.is_empty() {
        return vec![];
    }
    // The user's own month-of-year distribution is the baseline: a diary
    // started in October, or a user who simply watches more in winter, must
    // not make every big genre look seasonal.
    let mut user_months = [0u32; 12];
    let mut by_genre: HashMap<&str, [u32; 12]> = HashMap::new();
    for row in rows {
        let m = local(tz, &row.watched_at).month0() as usize;
        user_months[m] += 1;
        for genre in &row.genres {
            by_genre.entry(genre).or_insert([0; 12])[m] += 1;
        }
    }
    let user_total = rows.len() as f64;

    let mut candidates: Vec<Insight> = by_genre
        .into_iter()
        .filter_map(|(genre, histogram)| {
            let genre_total: u32 = histogram.iter().sum();
            if (genre_total as usize) < MIN_GENRE_SEASON_ROWS {
                return None;
            }
            // Month with the largest lift over the user's share for that
            // month; smallest month number wins ties — deterministic output.
            let (peak_month, multiplier) = histogram
                .iter()
                .enumerate()
                .filter(|(_, c)| **c >= MIN_SEASON_PEAK_ROWS)
                .map(|(i, &c)| {
                    let genre_share = f64::from(c) / f64::from(genre_total);
                    let user_share = f64::from(user_months[i]) / user_total;
                    (i as u32 + 1, round2(genre_share / user_share))
                })
                .fold(None, |best: Option<(u32, f64)>, cur| match best {
                    Some(b) if b.1 >= cur.1 => Some(b),
                    _ => Some(cur),
                })?;
            if multiplier < MIN_SEASON_MULTIPLIER {
                return None;
            }
            Some(Insight::Seasonality {
                genre: genre.to_string(),
                month: peak_month,
                multiplier,
                sample_count: genre_total,
            })
        })
        .collect();

    candidates.sort_by(strongest_first);
    candidates.truncate(MAX_PER_CATEGORY);
    candidates
}

// ── Medium timing ────────────────────────────────────────────────────────────

fn medium_timings<Tz: TimeZone>(rows: &[WatchRow], tz: &Tz) -> Vec<Insight> {
    if rows.is_empty() {
        return vec![];
    }
    let with_medium_count = rows.iter().filter(|r| r.watch_medium.is_some()).count();
    let missing_share = 1.0 - with_medium_count as f64 / rows.len() as f64;
    if missing_share > MAX_MISSING_MEDIUM_SHARE {
        return vec![];
    }

    // (total watches, weekend watches) per medium.
    let mut by_medium: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut weekend_total = 0usize;
    for row in rows.iter().filter(|r| r.watch_medium.is_some()) {
        let weekend = matches!(
            local(tz, &row.watched_at).weekday(),
            Weekday::Sat | Weekday::Sun
        );
        weekend_total += usize::from(weekend);
        let entry = by_medium
            .entry(row.watch_medium.as_deref().expect("filtered above"))
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += usize::from(weekend);
    }

    let baseline_share = weekend_total as f64 / with_medium_count as f64;
    // No weekend watches at all → no baseline to lift against.
    if baseline_share == 0.0 {
        return vec![];
    }

    let mut candidates: Vec<Insight> = by_medium
        .into_iter()
        .filter(|(_, (count, _))| *count >= MIN_MEDIUM_ROWS)
        .filter_map(|(medium, (count, weekend))| {
            let weekend_share = weekend as f64 / count as f64;
            let multiplier = round2(weekend_share / baseline_share);
            if multiplier < MIN_MEDIUM_MULTIPLIER {
                return None;
            }
            Some(Insight::MediumTiming {
                medium: medium.to_string(),
                multiplier,
                weekend_share: round2(weekend_share),
                baseline_share: round2(baseline_share),
                sample_count: count as u32,
            })
        })
        .collect();

    candidates.sort_by(strongest_first);
    candidates.truncate(MAX_PER_CATEGORY);
    candidates
}

// ── Rhythm ───────────────────────────────────────────────────────────────────

fn rhythm<Tz: TimeZone>(rows: &[WatchRow], tz: &Tz) -> Vec<Insight> {
    // Import noise is detected on the stored value: date-only imports land at
    // exactly 00:00 as stored, which no longer reads as midnight once shifted
    // into the viewer's zone.
    let real: Vec<NaiveDateTime> = rows
        .iter()
        .filter(|r| !(r.watched_at.hour() == 0 && r.watched_at.minute() == 0))
        .map(|r| local(tz, &r.watched_at))
        .collect();
    let midnight_share = 1.0 - real.len() as f64 / rows.len().max(1) as f64;
    if rows.is_empty() || real.len() < MIN_RHYTHM_ROWS || midnight_share > MAX_MIDNIGHT_SHARE {
        return vec![];
    }

    let mut hours = [0u32; 24];
    let mut days = [0u32; 7]; // index 0 = Monday (chrono weekday number − 1)
    for at in &real {
        hours[at.hour() as usize] += 1;
        days[at.weekday().number_from_monday() as usize - 1] += 1;
    }

    // Circular 3-hour window with the most watches; smallest start hour wins ties.
    let mut best_start = 0usize;
    let mut best_count = 0u32;
    for start in 0..24 {
        let count: u32 = (0..RHYTHM_WINDOW_HOURS)
            .map(|o| hours[(start + o as usize) % 24])
            .sum();
        if count > best_count {
            best_count = count;
            best_start = start;
        }
    }

    // Days carrying at least 60% of the busiest day's load, strongest first,
    // capped at three — "mostly Tue–Thu" without the tail of noise.
    let max_day = days.iter().copied().max().unwrap_or(0);
    let mut top_days: Vec<(u32, u32)> = days
        .into_iter()
        .enumerate()
        .filter(|(_, c)| *c > 0 && f64::from(*c) >= f64::from(max_day) * 0.6)
        .map(|(i, c)| (i as u32 + 1, c))
        .collect();
    // Count descending, then day number ascending — deterministic.
    top_days.sort_unstable_by_key(|&(day, count)| (std::cmp::Reverse(count), day));
    let top_days: Vec<u32> = top_days.into_iter().map(|(day, _)| day).take(3).collect();

    vec![Insight::Rhythm {
        peak_hour_start: best_start as u32,
        peak_hour_end: (best_start as u32 + RHYTHM_WINDOW_HOURS) % 24,
        top_days,
        sample_count: real.len() as u32,
    }]
}

// ── Shared helpers ───────────────────────────────────────────────────────────

/// Strength = effect size scaled by evidence volume. A 1.6× lift on 60 rows
/// outranks 1.7× on 3; a raw lift would not.
fn strength(effect: f64, sample: usize) -> f64 {
    effect * (1.0 + (sample as f64).ln())
}

fn strength_of(insight: &Insight) -> f64 {
    match insight {
        Insight::RatingLift {
            delta,
            sample_count,
            ..
        } => strength(delta.abs(), *sample_count as usize),
        Insight::Seasonality {
            multiplier,
            sample_count,
            ..
        }
        | Insight::MediumTiming {
            multiplier,
            sample_count,
            ..
        } => strength(multiplier - 1.0, *sample_count as usize),
        Insight::Rhythm { .. } => 0.0,
    }
}

fn label_of(insight: &Insight) -> &str {
    match insight {
        Insight::RatingLift { genre, .. } | Insight::Seasonality { genre, .. } => genre,
        Insight::MediumTiming { medium, .. } => medium,
        Insight::Rhythm { .. } => "",
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
#[path = "tests/insights_analyzer.rs"]
mod tests;
