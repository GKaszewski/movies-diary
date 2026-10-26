use chrono::{Datelike, FixedOffset, NaiveDate, Utc, Weekday};
use uuid::Uuid;

use domain::models::{Insight, WatchRow};

use domain::services::insights_analyzer::build_insights;

fn watch(genres: &[&str], rating: u8, date: NaiveDate, hour: u32) -> WatchRow {
    WatchRow {
        movie_id: Uuid::new_v4(),
        rating,
        watched_at: date.and_hms_opt(hour, 30, 0).unwrap(),
        watch_medium: Some("streaming".to_string()),
        genres: genres.iter().map(|g| g.to_string()).collect(),
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn kinds(insights: &[Insight]) -> Vec<&str> {
    insights
        .iter()
        .map(|i| match i {
            Insight::RatingLift { .. } => "rating_lift",
            Insight::Seasonality { .. } => "seasonality",
            Insight::MediumTiming { .. } => "medium_timing",
            Insight::Rhythm { .. } => "rhythm",
        })
        .collect()
}

// ── Rating lift ──────────────────────────────────────────────────────────────

#[test]
fn rating_lift_compares_genre_to_personal_average() {
    // 12 thrillers rated 5, 12 comedies rated 2 — user average 3.5.
    let mut rows: Vec<WatchRow> = (0..12)
        .map(|i| watch(&["Thriller"], 5, date(2024, 1, 1 + i), 20))
        .collect();
    rows.extend((0..12).map(|i| watch(&["Comedy"], 2, date(2024, 3, 1 + i), 20)));

    let insights = build_insights(&rows, &Utc);
    let thrill = insights.iter().find_map(|i| match i {
        Insight::RatingLift {
            genre,
            delta,
            sample_count,
            ..
        } if genre == "Thriller" => Some((*delta, *sample_count)),
        _ => None,
    });
    let (delta, count) = thrill.expect("Thriller lift expected");
    assert!((delta - 1.5).abs() < 1e-9, "delta was {delta}");
    assert_eq!(count, 12);

    let comedy = insights.iter().find_map(|i| match i {
        Insight::RatingLift { genre, delta, .. } if genre == "Comedy" => Some(*delta),
        _ => None,
    });
    assert!((comedy.expect("Comedy lift expected") + 1.5).abs() < 1e-9);
}

#[test]
fn rating_lift_ignores_rewatches() {
    // 10 first-time dramas rated 3 (baseline), 5 thrillers rated 5…
    let mut rows: Vec<WatchRow> = (0..10)
        .map(|i| watch(&["Drama"], 3, date(2024, 1, 1 + i), 20))
        .collect();
    let mut thriller_ids: Vec<uuid::Uuid> = Vec::new();
    for i in 0..5 {
        let row = watch(&["Thriller"], 5, date(2024, 2, 1 + i), 20);
        thriller_ids.push(row.movie_id);
        rows.push(row);
    }
    // …and 3 rewatches of those same thrillers rated 1, which must not count.
    for (i, id) in thriller_ids.iter().enumerate() {
        let mut rewatch = watch(&["Thriller"], 1, date(2024, 6, 1 + i as u32), 20);
        rewatch.movie_id = *id;
        rows.push(rewatch);
    }

    let insights = build_insights(&rows, &Utc);
    let thrill = insights.iter().find_map(|i| match i {
        Insight::RatingLift {
            genre,
            delta,
            sample_count,
            ..
        } if genre == "Thriller" => Some((*delta, *sample_count)),
        _ => None,
    });
    let (delta, count) = thrill.expect("Thriller lift expected");
    // 15 first-time rows: user_avg = (10*3 + 5*5)/15 = 55/15 ≈ 3.67,
    // genre_avg = 5.0, delta ≈ 1.33.
    assert!((delta - 1.33).abs() < 0.01, "delta was {delta}");
    assert_eq!(count, 5, "rewatches must not join the genre sample");
}

#[test]
fn rating_lift_below_sample_threshold_is_absent() {
    let mut rows: Vec<WatchRow> = (0..10)
        .map(|i| watch(&["Drama"], 3, date(2024, 1, 1 + i), 20))
        .collect();
    // Only 4 thriller ratings — below MIN_GENRE_RATINGS, so no matter how
    // high they are rated, the genre must not yield an insight.
    rows.extend((0..4).map(|i| watch(&["Thriller"], 5, date(2024, 2, 1 + i), 20)));

    let insights = build_insights(&rows, &Utc);
    assert!(
        !insights
            .iter()
            .any(|i| matches!(i, Insight::RatingLift { genre, .. } if genre == "Thriller")),
        "a genre with 4 ratings must not yield a lift"
    );
}

#[test]
fn rating_lift_below_delta_threshold_is_absent() {
    // 5 thrillers rated 4; dramas mix to put the personal average at 3.85,
    // so the thriller delta is 0.15 < 0.3.
    let mut rows: Vec<WatchRow> = (0..12)
        .map(|i| watch(&["Drama"], 4, date(2024, 1, 1 + i % 28), 20))
        .collect();
    rows.extend((0..3).map(|i| watch(&["Drama"], 3, date(2024, 1, 13 + i), 20)));
    rows.extend((0..5).map(|i| watch(&["Thriller"], 4, date(2024, 2, 1 + i), 20)));

    let insights = build_insights(&rows, &Utc);
    assert!(!kinds(&insights).contains(&"rating_lift"));
}

// ── Seasonality ──────────────────────────────────────────────────────────────

#[test]
fn seasonality_detects_month_peak_against_own_monthly_baseline() {
    // 16 horrors: 10 in October across two years, 6 scattered.
    let mut rows: Vec<WatchRow> = Vec::new();
    for i in 0..10u32 {
        let year = if i < 5 { 2023 } else { 2024 };
        rows.push(watch(&["Horror"], 4, date(year, 10, 1 + i % 27), 21));
    }
    for (m, d) in [(1u32, 3u32), (2, 7), (4, 11), (6, 15), (8, 19), (12, 21)] {
        rows.push(watch(&["Horror"], 4, date(2024, m, d), 21));
    }
    // A perfectly spread genre (2 watches per month) must NOT appear.
    rows.extend((0..24).map(|i| watch(&["Drama"], 3, date(2024, 1 + i % 12, 5), 21)));

    let insights = build_insights(&rows, &Utc);
    let horror = insights.iter().find_map(|i| match i {
        Insight::Seasonality {
            genre,
            month,
            multiplier,
            sample_count,
            ..
        } if genre == "Horror" => Some((*month, *multiplier, *sample_count)),
        _ => None,
    });
    let (month, multiplier, count) = horror.expect("Horror seasonality expected");
    assert_eq!(month, 10);
    // Horror: 10 of 16 watches in October (62.5%); the whole diary puts
    // 12 of 40 watches there (30%) → 0.625 / 0.3 ≈ 2.08× the user's own share.
    assert!(
        (multiplier - 2.08).abs() < 1e-9,
        "multiplier was {multiplier}"
    );
    assert_eq!(count, 16);

    assert!(
        !insights
            .iter()
            .any(|i| matches!(i, Insight::Seasonality { genre, .. } if genre == "Drama")),
        "a spread-out genre must not yield a seasonality insight"
    );
}

#[test]
fn seasonality_ignores_when_the_diary_itself_is_concentrated() {
    // A diary started in October: every genre peaks in Oct/Nov only because
    // the user logged nothing else. Relative to their own months, nothing is
    // seasonal.
    let mut rows: Vec<WatchRow> = Vec::new();
    for i in 0..20u32 {
        let month = if i % 2 == 0 { 10 } else { 11 };
        rows.push(watch(&["Drama"], 3, date(2024, month, 1 + i), 21));
        rows.push(watch(&["Comedy"], 3, date(2024, month, 1 + i), 21));
    }
    assert!(!kinds(&build_insights(&rows, &Utc)).contains(&"seasonality"));
}

#[test]
fn seasonality_below_sample_threshold_is_absent() {
    let rows: Vec<WatchRow> = (0..14)
        .map(|i| watch(&["Horror"], 4, date(2024, 10, 1 + i), 21))
        .collect();
    assert!(!kinds(&build_insights(&rows, &Utc)).contains(&"seasonality"));
}

// ── Medium timing ────────────────────────────────────────────────────────────

#[test]
fn medium_timing_detects_weekend_lift() {
    let saturday = date(2024, 3, 9); // a Saturday
    assert_eq!(saturday.weekday(), Weekday::Sat);
    let friday = date(2024, 3, 8);

    let mut rows: Vec<WatchRow> = Vec::new();
    // 6 cinema watches, 5 of them on Saturdays.
    let cinema_days = [
        saturday,
        saturday.with_day(16).unwrap(),
        saturday.with_day(23).unwrap(),
        saturday.with_day(30).unwrap(),
        date(2024, 4, 6), // next month's first Saturday
        friday,
    ];
    for day in cinema_days {
        let mut row = watch(&["Drama"], 4, day, 19);
        row.watch_medium = Some("cinema".to_string());
        rows.push(row);
    }
    // 10 streaming watches on weekdays only (two Mon–Fri weeks) to keep the
    // baseline weekend share driven by cinema alone.
    for i in 0..10u32 {
        let mut row = watch(&["Drama"], 4, friday, 21);
        row.watch_medium = Some("streaming".to_string());
        row.watched_at = date(2024, 3, 4 + i % 5 + 7 * (i / 5))
            .and_hms_opt(21, 0, 0)
            .unwrap();
        rows.push(row);
    }

    let insights = build_insights(&rows, &Utc);
    let cinema = insights.iter().find_map(|i| match i {
        Insight::MediumTiming {
            medium,
            multiplier,
            sample_count,
            ..
        } if medium == "cinema" => Some((*multiplier, *sample_count)),
        _ => None,
    });
    let (multiplier, count) = cinema.expect("cinema timing expected");
    // Baseline weekend share = 5/16; cinema share = 5/6 → lift = (5/6)/(5/16) ≈ 2.67.
    assert!(
        (multiplier - 2.67).abs() < 0.01,
        "multiplier was {multiplier}"
    );
    assert_eq!(count, 6);
}

#[test]
fn medium_timing_skipped_when_medium_mostly_missing() {
    let mut rows: Vec<WatchRow> = Vec::new();
    let first_saturday = date(2024, 3, 9);
    for i in 0..6u32 {
        let mut row = watch(
            &["Drama"],
            4,
            first_saturday + chrono::Duration::weeks(i64::from(i)),
            20,
        );
        row.watch_medium = Some("cinema".to_string());
        rows.push(row);
    }
    for i in 0..10u32 {
        let mut row = watch(&["Drama"], 4, date(2024, 3, 1 + i), 20);
        row.watch_medium = None; // imports without a medium
        rows.push(row);
    }

    assert!(!kinds(&build_insights(&rows, &Utc)).contains(&"medium_timing"));
}

// ── Rhythm ───────────────────────────────────────────────────────────────────

#[test]
fn rhythm_finds_peak_window_and_days() {
    // 12 watches at 22:30: five Tuesdays, four Wednesdays, three Thursdays.
    let first_tuesday = date(2024, 3, 5);
    assert_eq!(first_tuesday.weekday(), Weekday::Tue);
    let mut rows: Vec<WatchRow> = Vec::new();
    for day in 0..3u32 {
        let count = 5 - day; // Tue ×5, Wed ×4, Thu ×3
        for week in 0..count {
            rows.push(watch(
                &["Drama"],
                4,
                first_tuesday
                    + chrono::Duration::days(i64::from(day))
                    + chrono::Duration::weeks(i64::from(week)),
                22,
            ));
        }
    }

    let insights = build_insights(&rows, &Utc);
    let rhythm = insights.iter().find_map(|i| match i {
        Insight::Rhythm {
            peak_hour_start,
            peak_hour_end,
            top_days,
            sample_count,
        } => Some((
            *peak_hour_start,
            *peak_hour_end,
            top_days.clone(),
            *sample_count,
        )),
        _ => None,
    });
    let (start, end, days, count) = rhythm.expect("rhythm expected");
    // All watches sit at hour 22; the earliest 3-hour window that covers
    // them starts at 20 (20, 21, 22).
    assert_eq!(start, 20);
    assert_eq!(end, 23);
    // Every day carries ≥60% of the busiest day (5): Tue(5) Wed(4) Thu(3).
    assert_eq!(days, vec![2, 3, 4], "Tue=2, Wed=3, Thu=4");
    assert_eq!(count, 12);
}

#[test]
fn rhythm_buckets_in_the_viewers_time_zone() {
    // 12 watches stored at 22:30 UTC on Tue/Wed/Thu are 00:30 the next day
    // for a viewer at UTC+2.
    let first_tuesday = date(2024, 3, 5);
    let mut rows: Vec<WatchRow> = Vec::new();
    for day in 0..3u32 {
        for week in 0..(5 - day) {
            rows.push(watch(
                &["Drama"],
                4,
                first_tuesday
                    + chrono::Duration::days(i64::from(day))
                    + chrono::Duration::weeks(i64::from(week)),
                22,
            ));
        }
    }
    let plus_two = FixedOffset::east_opt(2 * 3600).unwrap();

    let insights = build_insights(&rows, &plus_two);
    let (start, end, days) = insights
        .iter()
        .find_map(|i| match i {
            Insight::Rhythm {
                peak_hour_start,
                peak_hour_end,
                top_days,
                ..
            } => Some((*peak_hour_start, *peak_hour_end, top_days.clone())),
            _ => None,
        })
        .expect("rhythm expected");
    // Local hour 0; the scan starts at 0, so the first window covering it is 0–3.
    assert_eq!((start, end), (0, 3));
    assert_eq!(days, vec![3, 4, 5], "Wed=3, Thu=4, Fri=5 locally");
}

#[test]
fn rhythm_import_noise_detected_before_zone_shift() {
    // Date-only imports are stored at 00:00 UTC; at UTC+2 they read 02:00,
    // but they are still noise.
    let rows: Vec<WatchRow> = (0..12u32)
        .map(|i| WatchRow {
            movie_id: Uuid::new_v4(),
            rating: 4,
            watched_at: date(2024, 1, 1 + i).and_hms_opt(0, 0, 0).unwrap(),
            watch_medium: None,
            genres: vec!["Drama".to_string()],
        })
        .collect();
    let plus_two = FixedOffset::east_opt(2 * 3600).unwrap();
    assert!(!kinds(&build_insights(&rows, &plus_two)).contains(&"rhythm"));
}

#[test]
fn rhythm_skipped_when_timestamps_are_import_noise() {
    let mut rows: Vec<WatchRow> = Vec::new();
    for i in 0..12u32 {
        // Exactly midnight — the signature of a CSV import with no time data.
        let row = WatchRow {
            movie_id: Uuid::new_v4(),
            rating: 4,
            watched_at: date(2024, 1, 1 + i).and_hms_opt(0, 0, 0).unwrap(),
            watch_medium: None,
            genres: vec!["Drama".to_string()],
        };
        rows.push(row);
    }

    assert!(!kinds(&build_insights(&rows, &Utc)).contains(&"rhythm"));
}

// ── Overall ──────────────────────────────────────────────────────────────────

#[test]
fn empty_diary_yields_nothing() {
    assert!(build_insights(&[], &Utc).is_empty());
}

#[test]
fn small_diary_yields_nothing() {
    let rows: Vec<WatchRow> = (0..4)
        .map(|i| watch(&["Drama"], 4, date(2024, 1, 1 + i), 20))
        .collect();
    assert!(build_insights(&rows, &Utc).is_empty());
}

#[test]
fn max_three_insights_per_category() {
    // Six genres, all with a strong rating lift — only the three strongest survive.
    let mut rows: Vec<WatchRow> = Vec::new();
    let genres = [
        ("Thriller", 5u8),
        ("Horror", 5),
        ("Sci-Fi", 5),
        ("Comedy", 5),
        ("Romance", 5),
        ("Documentary", 5),
    ];
    for (genre, rating) in genres {
        rows.extend((0..5).map(|i| watch(&[genre], rating, date(2024, 1, 1 + i), 20)));
    }
    // Baseline dramas rated 1 make every lift large.
    rows.extend((0..10).map(|i| watch(&["Drama"], 1, date(2024, 3, 1 + i), 20)));

    let insights = build_insights(&rows, &Utc);
    let rating_lifts = insights
        .iter()
        .filter(|i| matches!(i, Insight::RatingLift { .. }))
        .count();
    assert_eq!(rating_lifts, 3);
}
