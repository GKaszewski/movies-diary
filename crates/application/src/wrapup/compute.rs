use std::collections::HashMap;

use chrono::Datelike;
use uuid::Uuid;

use crate::context::AppContext;
use crate::wrapup::queries::ComputeWrapUpQuery;
use domain::errors::DomainError;
use domain::models::wrapup::*;
use domain::ports::WrapUpMovieRow;

pub async fn execute(
    ctx: &AppContext,
    query: ComputeWrapUpQuery,
) -> Result<WrapUpReport, DomainError> {
    let rows = ctx
        .repos
        .wrapup_stats
        .get_reviews_with_profiles(&query.scope, &query.date_range)
        .await?;

    Ok(build_report(query.scope, query.date_range, &rows))
}

fn build_report(
    scope: WrapUpScope,
    date_range: DateRange,
    rows: &[WrapUpMovieRow],
) -> WrapUpReport {
    let total_movies = rows.len() as u32;

    let total_watch_time_minutes: u32 = rows.iter().filter_map(|r| r.runtime_minutes).sum();

    let avg_rating = if rows.is_empty() {
        None
    } else {
        Some(rows.iter().map(|r| r.rating as f64).sum::<f64>() / rows.len() as f64)
    };

    let rating_distribution = {
        let mut dist = [0u32; 5];
        for r in rows {
            let idx = (r.rating as usize).saturating_sub(1).min(4);
            dist[idx] += 1;
        }
        dist
    };

    let movies_per_month = compute_movies_per_month(rows);
    let busiest_month = movies_per_month
        .iter()
        .max_by_key(|m| m.count)
        .map(|m| m.label.clone());

    let busiest_day_of_week = compute_busiest_day(rows);

    let (longest_movie, shortest_movie) = compute_runtime_extremes(rows);
    let (highest_rated_movie, lowest_rated_movie) = compute_rating_extremes(rows);
    let (first_movie_of_period, last_movie_of_period) = compute_chronological_extremes(rows);
    let (oldest_movie, newest_movie) = compute_year_extremes(rows);

    let (top_directors, director_diversity) = compute_director_stats(rows);
    let (top_actors, actor_diversity, top_cast_profile_paths) = compute_actor_stats(rows);

    let (top_genres, genre_diversity, highest_rated_genre, lowest_rated_genre) =
        compute_genre_stats(rows);
    let top_keywords = compute_keyword_stats(rows);

    let (total_budget_watched, avg_budget) = compute_budget_stats(rows);
    let language_distribution = compute_language_stats(rows);

    let (total_rewatches, most_rewatched_movie, avg_rating_change_on_rewatch) =
        compute_rewatch_stats(rows);

    let (most_active_user, most_watched_movie_global, total_users_active) =
        if matches!(scope, WrapUpScope::Global) {
            compute_global_stats(rows)
        } else {
            (None, None, None)
        };

    let poster_paths: Vec<String> = rows.iter().filter_map(|r| r.poster_path.clone()).collect();

    WrapUpReport {
        scope,
        date_range,
        generated_at: chrono::Utc::now(),
        total_movies,
        total_watch_time_minutes,
        movies_per_month,
        busiest_month,
        busiest_day_of_week,
        avg_rating,
        rating_distribution,
        longest_movie,
        shortest_movie,
        top_directors,
        top_actors,
        director_diversity,
        actor_diversity,
        top_genres,
        genre_diversity,
        highest_rated_genre,
        lowest_rated_genre,
        top_keywords,
        total_budget_watched,
        avg_budget,
        language_distribution,
        oldest_movie,
        newest_movie,
        total_rewatches,
        most_rewatched_movie,
        avg_rating_change_on_rewatch,
        highest_rated_movie,
        lowest_rated_movie,
        first_movie_of_period,
        last_movie_of_period,
        most_active_user,
        most_watched_movie_global,
        total_users_active,
        poster_paths,
        top_cast_profile_paths,
    }
}

fn movie_ref(r: &WrapUpMovieRow) -> MovieRef {
    MovieRef {
        title: r.title.clone(),
        year: r.release_year,
        runtime_minutes: r.runtime_minutes,
        poster_path: r.poster_path.clone(),
    }
}

fn compute_movies_per_month(rows: &[WrapUpMovieRow]) -> Vec<MonthCount> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for r in rows {
        let ym = r.watched_at.format("%Y-%m").to_string();
        *counts.entry(ym).or_default() += 1;
    }
    let mut result: Vec<MonthCount> = counts
        .into_iter()
        .map(|(ym, count)| {
            let label = chrono::NaiveDate::parse_from_str(&format!("{ym}-01"), "%Y-%m-%d")
                .map(|d| d.format("%B %Y").to_string())
                .unwrap_or_else(|_| ym.clone());
            MonthCount {
                year_month: ym,
                label,
                count,
            }
        })
        .collect();
    result.sort_by(|a, b| a.year_month.cmp(&b.year_month));
    result
}

fn compute_busiest_day(rows: &[WrapUpMovieRow]) -> Option<String> {
    if rows.is_empty() {
        return None;
    }
    let mut day_counts = [0u32; 7];
    for r in rows {
        let weekday = r.watched_at.date().weekday().num_days_from_monday() as usize;
        day_counts[weekday] += 1;
    }
    let max_idx = day_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| *c)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let names = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    Some(names[max_idx].to_string())
}

fn compute_runtime_extremes(rows: &[WrapUpMovieRow]) -> (Option<MovieRef>, Option<MovieRef>) {
    let with_runtime: Vec<_> = rows
        .iter()
        .filter(|r| r.runtime_minutes.is_some())
        .collect();
    let longest = with_runtime
        .iter()
        .max_by_key(|r| r.runtime_minutes.unwrap_or(0))
        .map(|r| movie_ref(r));
    let shortest = with_runtime
        .iter()
        .min_by_key(|r| r.runtime_minutes.unwrap_or(u32::MAX))
        .map(|r| movie_ref(r));
    (longest, shortest)
}

fn compute_rating_extremes(rows: &[WrapUpMovieRow]) -> (Option<MovieRef>, Option<MovieRef>) {
    let highest = rows.iter().max_by_key(|r| r.rating).map(movie_ref);
    let lowest = rows.iter().min_by_key(|r| r.rating).map(movie_ref);
    (highest, lowest)
}

fn compute_chronological_extremes(rows: &[WrapUpMovieRow]) -> (Option<MovieRef>, Option<MovieRef>) {
    let first = rows.iter().min_by_key(|r| r.watched_at).map(movie_ref);
    let last = rows.iter().max_by_key(|r| r.watched_at).map(movie_ref);
    (first, last)
}

fn compute_year_extremes(rows: &[WrapUpMovieRow]) -> (Option<MovieRef>, Option<MovieRef>) {
    let oldest = rows.iter().min_by_key(|r| r.release_year).map(movie_ref);
    let newest = rows.iter().max_by_key(|r| r.release_year).map(movie_ref);
    (oldest, newest)
}

fn compute_director_stats(rows: &[WrapUpMovieRow]) -> (Vec<PersonStat>, u32) {
    let mut director_movies: HashMap<String, Vec<u8>> = HashMap::new();
    for r in rows {
        if let Some(ref dir) = r.director {
            director_movies
                .entry(dir.clone())
                .or_default()
                .push(r.rating);
        }
    }
    let diversity = director_movies.len() as u32;
    let mut stats: Vec<PersonStat> = director_movies
        .into_iter()
        .map(|(name, ratings)| {
            let count = ratings.len() as u32;
            let avg = ratings.iter().map(|&r| r as f64).sum::<f64>() / ratings.len() as f64;
            PersonStat {
                name,
                count,
                avg_rating: avg,
            }
        })
        .collect();
    stats.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(b.avg_rating.total_cmp(&a.avg_rating))
    });
    stats.truncate(5);
    (stats, diversity)
}

fn compute_actor_stats(rows: &[WrapUpMovieRow]) -> (Vec<PersonStat>, u32, Vec<String>) {
    let mut actor_movies: HashMap<String, Vec<u8>> = HashMap::new();
    let mut actor_profiles: HashMap<String, Option<String>> = HashMap::new();
    for r in rows {
        for (i, (name, billing)) in r.cast_names.iter().enumerate() {
            if *billing <= 3 {
                actor_movies.entry(name.clone()).or_default().push(r.rating);
                if let Some(path) = r.cast_profile_paths.get(i) {
                    actor_profiles
                        .entry(name.clone())
                        .or_insert_with(|| path.clone());
                }
            }
        }
    }
    let diversity = actor_movies.len() as u32;
    let mut stats: Vec<PersonStat> = actor_movies
        .into_iter()
        .map(|(name, ratings)| {
            let count = ratings.len() as u32;
            let avg = ratings.iter().map(|&r| r as f64).sum::<f64>() / ratings.len() as f64;
            PersonStat {
                name,
                count,
                avg_rating: avg,
            }
        })
        .collect();
    stats.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then(b.avg_rating.total_cmp(&a.avg_rating))
    });
    stats.truncate(5);
    let profile_paths: Vec<String> = stats
        .iter()
        .filter_map(|s| actor_profiles.get(&s.name)?.clone())
        .collect();
    (stats, diversity, profile_paths)
}

fn compute_genre_stats(
    rows: &[WrapUpMovieRow],
) -> (Vec<GenreStat>, u32, Option<String>, Option<String>) {
    let mut genre_ratings: HashMap<String, Vec<u8>> = HashMap::new();
    for r in rows {
        for genre in &r.genres {
            genre_ratings
                .entry(genre.clone())
                .or_default()
                .push(r.rating);
        }
    }
    let diversity = genre_ratings.len() as u32;
    let mut stats: Vec<GenreStat> = genre_ratings
        .into_iter()
        .map(|(genre, ratings)| {
            let count = ratings.len() as u32;
            let avg = ratings.iter().map(|&r| r as f64).sum::<f64>() / ratings.len() as f64;
            GenreStat {
                genre,
                count,
                avg_rating: avg,
            }
        })
        .collect();
    stats.sort_by_key(|s| std::cmp::Reverse(s.count));
    let highest = stats
        .iter()
        .max_by(|a, b| a.avg_rating.total_cmp(&b.avg_rating))
        .map(|g| g.genre.clone());
    let lowest = stats
        .iter()
        .filter(|g| g.count >= 3)
        .min_by(|a, b| a.avg_rating.total_cmp(&b.avg_rating))
        .map(|g| g.genre.clone());
    stats.truncate(5);
    (stats, diversity, highest, lowest)
}

fn compute_keyword_stats(rows: &[WrapUpMovieRow]) -> Vec<KeywordStat> {
    let mut kw_counts: HashMap<String, u32> = HashMap::new();
    for r in rows {
        for kw in &r.keywords {
            *kw_counts.entry(kw.clone()).or_default() += 1;
        }
    }
    let mut stats: Vec<KeywordStat> = kw_counts
        .into_iter()
        .map(|(keyword, count)| KeywordStat { keyword, count })
        .collect();
    stats.sort_by_key(|s| std::cmp::Reverse(s.count));
    stats.truncate(20);
    stats
}

fn compute_budget_stats(rows: &[WrapUpMovieRow]) -> (Option<i64>, Option<i64>) {
    let budgets: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.budget_usd)
        .filter(|&b| b > 0)
        .collect();
    if budgets.is_empty() {
        return (None, None);
    }
    let total: i64 = budgets.iter().sum();
    let avg = total / budgets.len() as i64;
    (Some(total), Some(avg))
}

fn compute_language_stats(rows: &[WrapUpMovieRow]) -> Vec<LangStat> {
    let mut lang_counts: HashMap<String, u32> = HashMap::new();
    for r in rows {
        if let Some(ref lang) = r.original_language {
            *lang_counts.entry(lang.clone()).or_default() += 1;
        }
    }
    let mut stats: Vec<LangStat> = lang_counts
        .into_iter()
        .map(|(language, count)| LangStat { language, count })
        .collect();
    stats.sort_by_key(|s| std::cmp::Reverse(s.count));
    stats
}

fn compute_rewatch_stats(rows: &[WrapUpMovieRow]) -> (u32, Option<MovieRef>, Option<f64>) {
    let mut movie_reviews: HashMap<Uuid, Vec<&WrapUpMovieRow>> = HashMap::new();
    for r in rows {
        movie_reviews.entry(r.movie_id).or_default().push(r);
    }

    let rewatched: Vec<_> = movie_reviews
        .iter()
        .filter(|(_, reviews)| reviews.len() > 1)
        .collect();

    let total_rewatches = rewatched.iter().map(|(_, rs)| rs.len() as u32 - 1).sum();

    let most_rewatched = rewatched
        .iter()
        .max_by_key(|(_, rs)| rs.len())
        .map(|(_, rs)| movie_ref(rs[0]));

    let avg_change = if rewatched.is_empty() {
        None
    } else {
        let changes: Vec<f64> = rewatched
            .iter()
            .filter_map(|(_, rs)| {
                let mut sorted: Vec<_> = rs.to_vec();
                sorted.sort_by_key(|r| r.watched_at);
                let first = sorted.first()?.rating as f64;
                let last = sorted.last()?.rating as f64;
                Some(last - first)
            })
            .collect();
        if changes.is_empty() {
            None
        } else {
            Some(changes.iter().sum::<f64>() / changes.len() as f64)
        }
    };

    (total_rewatches, most_rewatched, avg_change)
}

fn compute_global_stats(
    rows: &[WrapUpMovieRow],
) -> (Option<UserRef>, Option<MovieRef>, Option<u32>) {
    if rows.is_empty() {
        return (None, None, None);
    }

    let mut user_counts: HashMap<Uuid, u32> = HashMap::new();
    for r in rows {
        *user_counts.entry(r.user_id).or_default() += 1;
    }
    let total_users_active = Some(user_counts.len() as u32);

    let most_active_user = user_counts
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(&uid, _)| UserRef {
            user_id: uid,
            display_name: String::new(),
        });

    let mut movie_counts: HashMap<Uuid, (u32, &WrapUpMovieRow)> = HashMap::new();
    for r in rows {
        movie_counts
            .entry(r.movie_id)
            .and_modify(|(c, _)| *c += 1)
            .or_insert((1, r));
    }
    let most_watched = movie_counts
        .into_iter()
        .max_by_key(|(_, (c, _))| *c)
        .map(|(_, (_, r))| movie_ref(r));

    (most_active_user, most_watched, total_users_active)
}

#[cfg(test)]
#[path = "tests/compute.rs"]
mod tests;
