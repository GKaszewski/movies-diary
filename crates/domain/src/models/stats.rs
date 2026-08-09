use super::review::DiaryEntry;

#[derive(Clone, Debug)]
pub struct UserStats {
    pub total_movies: i64,
    pub avg_rating: Option<f64>,
    pub favorite_director: Option<String>,
    pub most_active_month: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MonthActivity {
    pub year_month: String,
    pub month_label: String,
    pub count: i64,
    pub entries: Vec<DiaryEntry>,
}

#[derive(Clone, Debug)]
pub struct MonthlyRating {
    pub year_month: String,
    pub month_label: String,
    pub avg_rating: f64,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct DirectorStat {
    pub director: String,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct GenreStat {
    pub genre: String,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct WatchMediumStat {
    pub medium: String,
    pub count: i64,
}

#[derive(Clone, Debug)]
pub struct UserTrends {
    pub monthly_ratings: Vec<MonthlyRating>,
    pub top_directors: Vec<DirectorStat>,
    pub max_director_count: i64,
    pub top_genres: Vec<GenreStat>,
    pub rating_distribution: [i64; 5],
    pub watch_medium_distribution: Vec<WatchMediumStat>,
    pub top_actors: Vec<ActorStat>,
}

#[derive(Clone, Debug)]
pub struct ActorAppearance {
    pub tmdb_person_id: u64,
    pub name: String,
    pub profile_path: Option<String>,
    pub billing_order: u32,
    pub movie_id: String,
}

#[derive(Clone, Debug)]
pub struct ActorStat {
    pub tmdb_person_id: u64,
    pub name: String,
    pub profile_path: Option<String>,
    pub movie_count: u32,
    pub score: f64,
}

const LEAD_BILLING_MAX: u32 = 2;
const SUPPORTING_BILLING_MIN: u32 = 3;
const SUPPORTING_BILLING_MAX: u32 = 9;

const LEAD_WEIGHT: f64 = 1.0;
const SUPPORTING_WEIGHT: f64 = 0.7;
const MINOR_WEIGHT: f64 = 0.2;

const TOP_ACTORS_LIMIT: usize = 5;

fn billing_weight(order: u32) -> f64 {
    match order {
        0..=LEAD_BILLING_MAX => LEAD_WEIGHT,
        SUPPORTING_BILLING_MIN..=SUPPORTING_BILLING_MAX => SUPPORTING_WEIGHT,
        _ => MINOR_WEIGHT,
    }
}

pub fn compute_top_actors(appearances: Vec<ActorAppearance>) -> Vec<ActorStat> {
    use std::collections::{HashMap, HashSet};

    struct ActorAccumulator {
        name: String,
        profile_path: Option<String>,
        score: f64,
        seen_movies: HashSet<String>,
    }

    let mut actors_by_id: HashMap<u64, ActorAccumulator> = HashMap::new();

    for appearance in appearances {
        let accumulator = actors_by_id
            .entry(appearance.tmdb_person_id)
            .or_insert_with(|| ActorAccumulator {
                name: appearance.name.clone(),
                profile_path: appearance.profile_path.clone(),
                score: 0.0,
                seen_movies: HashSet::new(),
            });
        if accumulator.seen_movies.insert(appearance.movie_id) {
            accumulator.score += billing_weight(appearance.billing_order);
        }
    }

    let mut ranked_actors: Vec<ActorStat> = actors_by_id
        .into_iter()
        .map(|(person_id, accumulator)| ActorStat {
            tmdb_person_id: person_id,
            name: accumulator.name,
            profile_path: accumulator.profile_path,
            movie_count: accumulator.seen_movies.len() as u32,
            score: accumulator.score,
        })
        .collect();

    ranked_actors.sort_by(|left, right| right.score.total_cmp(&left.score));
    ranked_actors.truncate(TOP_ACTORS_LIMIT);
    ranked_actors
}

#[derive(Clone, Debug)]
pub struct MovieStats {
    pub total_count: u64,
    pub avg_rating: Option<f64>,
    pub federated_count: u64,
    pub rating_histogram: [u64; 5], // index 0 = 1★, index 4 = 5★
}

#[cfg(test)]
#[path = "tests/stats.rs"]
mod tests;
