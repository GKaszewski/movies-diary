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
pub struct UserTrends {
    pub monthly_ratings: Vec<MonthlyRating>,
    pub top_directors: Vec<DirectorStat>,
    pub max_director_count: i64,
}

#[derive(Clone, Debug)]
pub struct MovieStats {
    pub total_count: u64,
    pub avg_rating: Option<f64>,
    pub federated_count: u64,
    pub rating_histogram: [u64; 5], // index 0 = 1★, index 4 = 5★
}
