#[derive(sqlx::FromRow)]
pub(crate) struct UserTotalsRow {
    pub total: i64,
    pub avg_rating: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct DirectorCountRow {
    pub director: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct MonthlyRatingRow {
    pub month: String,
    pub avg_rating: f64,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct GenreCountRow {
    pub genre: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct RatingDistRow {
    pub rating: i64,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct WatchMediumCountRow {
    pub watch_medium: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub(crate) struct ActorAppearanceRow {
    pub tmdb_person_id: i64,
    pub name: String,
    pub profile_path: Option<String>,
    pub billing_order: i32,
    pub movie_id: String,
}
